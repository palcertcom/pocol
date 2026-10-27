//! MODEXP (address 5, EIP-198 with EIP-2565 gas) with an exact fast path for a modulus that fits one 64-bit
//! word (D-2 measurement, A13-DERIVATION §3b: those shapes are the slowest per gas, up to ~157 ns per gas).
//! Every input that does not take the fast path, and every input revm would refuse, is handed to revm's own
//! `berlin_run` unchanged, so the gas, the errors and the other results are revm's by construction. On the
//! fast path the input is parsed exactly as revm's `run_inner` parses it, the gas is revm's
//! `berlin_gas_calc`, and the output is the number left-padded to the modulus length, as revm pads aurora's.
//! The value is computed as base^exp mod m with m = 2^t·q, q odd:
//! - mod q by Montgomery multiplication on one word;
//! - mod 2^t directly: an even base to an exponent of at least t is 0, an odd base's exponent is reduced
//!   mod 2^64 (the multiplicative group mod 2^t has exponent dividing 2^(t-1));
//! - joined by the Chinese remainder theorem.
//! The tests compare it with revm on random and edge inputs.
use revm::precompile::{modexp::{berlin_gas_calc,berlin_run as revm_berlin_run},utilities::{left_pad,left_pad_vec,right_pad_vec,right_pad_with_offset},
    Precompile,PrecompileOutput,PrecompileResult,PrecompileWithAddress,u64_to_address};
use revm::primitives::{Bytes,U256};

/// The precompile pocol's EVM runs at address 5 (in place of revm's BERLIN entry).
pub const BERLIN:PrecompileWithAddress=PrecompileWithAddress(u64_to_address(5),Precompile::Standard(berlin_run));

pub fn berlin_run(input:&Bytes,gas_limit:u64)->PrecompileResult{fast(input,gas_limit).map_or_else(||revm_berlin_run(input,gas_limit),Ok)}

/// The fast path's result, or None for every input it does not take (revm then runs it). It takes only inputs
/// revm accepts: min gas and the gas cost within `gas_limit`, lengths that fit usize, and a modulus value of
/// 2^64 - 1 or less (0 and 1 included).
fn fast(input:&[u8],gas_limit:u64)->Option<PrecompileOutput>{
    const MIN_GAS:u64=200;const HEADER:usize=96;
    if MIN_GAS>gas_limit{return None}
    let base_len=usize::try_from(U256::from_be_bytes(right_pad_with_offset::<32>(input,0).into_owned())).ok()?;
    let exp_len=usize::try_from(U256::from_be_bytes(right_pad_with_offset::<32>(input,32).into_owned())).ok()?;
    let mod_len=usize::try_from(U256::from_be_bytes(right_pad_with_offset::<32>(input,64).into_owned())).ok()?;
    if base_len==0&&mod_len==0{return None}
    let input=input.get(HEADER..).unwrap_or_default();
    let exp_highp={let p=right_pad_with_offset::<32>(input,base_len);U256::from_be_bytes(left_pad::<32>(&p[..exp_len.min(32)]).into_owned())};
    let gas=berlin_gas_calc(base_len as u64,exp_len as u64,mod_len as u64,&exp_highp);
    if gas>gas_limit{return None}
    // The modulus starts at base_len + exp_len in the (right-padded) data; only a value of one word is taken.
    let mod_start=base_len.checked_add(exp_len)?;
    let modulus=right_pad_vec(input.get(mod_start.min(input.len())..).unwrap_or_default(),mod_len);
    let m=word(&modulus)?;
    let total=mod_start.checked_add(mod_len)?;
    let data=right_pad_vec(input,total);
    let (base,rest)=data.split_at(base_len);let (exp,_)=rest.split_at(exp_len);
    let r=modpow_word(base,exp,m);
    let bytes=r.to_be_bytes();let lead=bytes.iter().take_while(|b|**b==0).count();
    // m = 0: aurora returns no bytes; otherwise the value's big-endian bytes ([0] for zero). Both pad to mod_len.
    let out:&[u8]=if m==0{&[]}else if r==0{&[0]}else{&bytes[lead..]};
    Some(PrecompileOutput::new(gas,left_pad_vec(out,mod_len).into_owned().into()))
}
/// The value of big-endian `b` if it fits one word.
fn word(b:&[u8])->Option<u64>{
    let s=b.iter().position(|x|*x!=0).map_or(&b[b.len()..],|i|&b[i..]);
    if s.len()>8{return None}
    Some(s.iter().fold(0u64,|a,x|(a<<8)|u64::from(*x)))
}
/// base^exp mod m, all big-endian; m = 0 gives 0 (the caller returns no bytes, as aurora does).
pub fn modpow_word(base:&[u8],exp:&[u8],m:u64)->u64{
    if m<=1{return 0}
    let exp=&exp[exp.iter().position(|x|*x!=0).unwrap_or(exp.len())..];
    let b=base.iter().fold(0u128,|a,x|((a<<8)|u128::from(*x))%u128::from(m)) as u64;
    if exp.is_empty(){return 1}
    let t=m.trailing_zeros();let q=m>>t;
    let x1=if q==1{0}else{Mont::new(q).pow(b%q,exp)};
    if t==0{return x1}
    let mask=if t==64{u64::MAX}else{(1u64<<t)-1};
    let b2=b&mask;
    let x2=if b2&1==0{
        // An even base: b2^e has at least e factors of 2, so it is 0 mod 2^t once e >= t (t <= 63 here).
        let small=exp.len()<=8&&{let e=exp.iter().fold(0u64,|a,x|(a<<8)|u64::from(*x));e<u64::from(t)};
        if small{let e=exp.iter().fold(0u64,|a,x|(a<<8)|u64::from(*x));pow_wrapping(b2,e)&mask}else{0}
    }else{
        // An odd base: b2^(2^(t-1)) = 1 mod 2^t, and 2^(t-1) divides 2^64, so only exp mod 2^64 counts.
        let tail=&exp[exp.len().saturating_sub(8)..];
        pow_wrapping(b2,tail.iter().fold(0u64,|a,x|(a<<8)|u64::from(*x)))&mask
    };
    if q==1{return x2}
    // CRT: x = x1 + q·((x2 - x1)·q^-1 mod 2^t); q·y < q·2^t = m, so nothing overflows.
    let y=x2.wrapping_sub(x1).wrapping_mul(inv_word(q))&mask;
    x1+q*y
}
/// x^e mod 2^64.
fn pow_wrapping(mut x:u64,mut e:u64)->u64{let mut r=1u64;while e>0{if e&1==1{r=r.wrapping_mul(x)}x=x.wrapping_mul(x);e>>=1;}r}
/// q^-1 mod 2^64 for odd q (Newton: each step doubles the correct low bits; 3 bits to start, 6 steps).
fn inv_word(q:u64)->u64{let mut x=q;for _ in 0..6{x=x.wrapping_mul(2u64.wrapping_sub(q.wrapping_mul(x)));}x}
/// Montgomery arithmetic mod an odd q > 1 with R = 2^64.
struct Mont{q:u64,neg_inv:u64,r2:u64}
impl Mont{
    fn new(q:u64)->Self{let r=(1u128<<64)%u128::from(q);Self{q,neg_inv:inv_word(q).wrapping_neg(),r2:((r*r)%u128::from(q)) as u64}}
    /// a·b·R^-1 mod q, for a, b < q.
    fn mul(&self,a:u64,b:u64)->u64{
        let t=u128::from(a)*u128::from(b);let m=(t as u64).wrapping_mul(self.neg_inv);
        let (s,c)=t.overflowing_add(u128::from(m)*u128::from(self.q));
        let u=((s>>64) as u64,c);
        // u = (t + m·q)/2^64 < 2q; with the carry it is u.0 + 2^64.
        if u.1||u.0>=self.q{u.0.wrapping_sub(self.q)}else{u.0}
    }
    /// x^e mod q, x < q, e big-endian without leading zeros (non-empty).
    fn pow(&self,x:u64,e:&[u8])->u64{
        let xm=self.mul(x,self.r2);let mut acc=self.mul(1,self.r2);
        for byte in e{for i in (0..8).rev(){acc=self.mul(acc,acc);if (byte>>i)&1==1{acc=self.mul(acc,xm);}}}
        self.mul(acc,1)
    }
}

#[cfg(test)] mod tests;
