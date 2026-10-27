use super::*;
use revm::precompile::modexp::berlin_run as revm_run;

struct Rng(u64);
impl Rng{fn next(&mut self)->u64{self.0^=self.0<<13;self.0^=self.0>>7;self.0^=self.0<<17;self.0}fn below(&mut self,n:u64)->u64{self.next()%n}}
fn word32(v:u64)->[u8;32]{let mut w=[0u8;32];w[24..].copy_from_slice(&v.to_be_bytes());w}
fn input(b:&[u8],e:&[u8],m:&[u8])->Vec<u8>{[&word32(b.len() as u64)[..],&word32(e.len() as u64),&word32(m.len() as u64),b,e,m].concat()}
fn bytes(r:&mut Rng,n:usize)->Vec<u8>{(0..n).map(|_|r.next() as u8).collect()}
/// A modulus of `len` bytes: a value of all its bytes (more than one word when len > 8, which the fast path must
/// leave to revm), or one of the one-word shapes the fast path must get right.
fn modulus(r:&mut Rng,len:usize)->Vec<u8>{
    let mut m=vec![0u8;len];if len==0{return m}
    // Every fifth modulus uses all its bytes (more than one word when len > 8): revm's path, not the fast one.
    if r.below(5)==0{let mut b=bytes(r,len);b[0]|=1;return b}
    let v:u64=match r.below(10){0=>0,1=>1,2=>2,3=>1u64<<r.below(64),4=>u64::MAX,5=>3,6=>(r.next()|1)>>r.below(64),
        7=>((r.next()|1)>>r.below(60))<<r.below(5),8=>r.next()&!1,_=>r.next()};
    let k=len.min(8);let be=v.to_be_bytes();m[len-k..].copy_from_slice(&be[8-k..]);m
}
fn exponent(r:&mut Rng)->Vec<u8>{
    match r.below(6){0=>vec![],1=>vec![0;r.below(40) as usize],2=>{let mut e=vec![0;r.below(8) as usize];e.push(r.below(70) as u8);e}
        3=>vec![0xff;[32,256,1024][r.below(3) as usize]],_=>{let n=r.below(80) as usize;bytes(r,n)}}
}
fn base(r:&mut Rng)->Vec<u8>{match r.below(5){0=>vec![],1=>vec![0;r.below(20) as usize],2=>{let n=r.below(9) as usize;let mut b=bytes(r,n);if let Some(x)=b.last_mut(){*x&=!1}b},_=>{let n=r.below(40) as usize;bytes(r,n)}}}

/// The precompile returns what revm's returns, Ok (gas and bytes) or Err, on random inputs that reach the fast
/// path and around it: moduli of 0 to 16 bytes with values 0, 1, powers of two, 2^64 - 1, odd, even with an odd
/// part; bases empty, zero, even, longer than the modulus; exponents empty, zero, below 70, all ones up to 1024
/// bytes; inputs cut short and gas limits below the minimum, below the cost and above it.
#[test] fn the_precompile_returns_what_revm_returns(){
    let mut r=Rng(0x5ad);let (mut fast_taken,mut cases)=(0u32,0u32);
    for _ in 0..60_000{
        let (b,e)=(base(&mut r),exponent(&mut r));let ml=[0,1,2,7,8,9,16][r.below(7) as usize];let m=modulus(&mut r,ml);
        let mut inp=input(&b,&e,&m);
        if r.below(10)==0{let cut=r.below(inp.len() as u64+1) as usize;inp.truncate(cut);}
        let gas=[0,199,200,1_000,100_000,30_000_000][r.below(6) as usize];
        let bytes=Bytes::from(inp);
        let (mine,theirs)=(berlin_run(&bytes,gas),revm_run(&bytes,gas));
        assert_eq!(format!("{mine:?}"),format!("{theirs:?}"),"input {bytes} gas {gas}");
        if fast(&bytes,gas).is_some(){fast_taken+=1}cases+=1;
    }
    assert!(fast_taken>cases/4,"positive control: the fast path ran in {fast_taken} of {cases} cases");
}
/// modpow_word against aurora (through revm, padded the same way) on full-range one-word moduli and
/// exponents, the shapes the D-2 measurement found slowest included.
#[test] fn modpow_word_equals_revm_on_one_word_moduli(){
    let mut r=Rng(0x6e9);
    for i in 0..20_000{
        let m=match i%4{0=>r.next(),1=>r.next()|1,2=>(r.next()|1)<<r.below(8),_=>r.next()&!1}.max(2);
        let bn=r.below(12) as usize;let b=bytes(&mut r,bn);let e=if i%50==0{vec![0xff;1024]}else{let n=r.below(40) as usize;bytes(&mut r,n)};
        let inp=Bytes::from(input(&b,&e,&m.to_be_bytes()));
        let theirs=revm_run(&inp,u64::MAX).unwrap().bytes;
        assert_eq!(&theirs[..],&modpow_word(&b,&e,m).to_be_bytes()[..],"m {m:#x} b {} e {}",Bytes::from(b.clone()),Bytes::from(e.clone()));
    }
}
/// The MODEXP shapes of the D-2 measurement (journal_measure's modexp_input), each checked against revm.
#[test] fn the_measured_shapes_match_revm(){
    for (bl,el,ml,kind) in [(8usize,1024usize,8usize,"even"),(8,4096,8,"even"),(8,1024,8,"odd"),(8,1024,8,"pow2"),(1,1024,1,"even"),(4,1024,4,"even"),(8,256,8,"even"),(8,32,8,"even")]{
        let fill=|n:usize,s:u8|->Vec<u8>{(0..n).map(|i|(i as u8).wrapping_mul(37).wrapping_add(s)).collect()};
        let mut b=fill(bl,11);b[0]|=0x80;if bl<=8{*b.last_mut().unwrap()&=!1}
        let mut m=fill(ml,101);m[0]|=0x80;
        match kind{"odd"=>*m.last_mut().unwrap()|=1,"even"=>{let x=m.last_mut().unwrap();*x=(*x|3)&!1}_=>{m=vec![0u8;ml];m[0]=0x80}}
        let inp=Bytes::from(input(&b,&vec![0xff;el],&m));
        assert!(fast(&inp,30_000_000).is_some(),"{kind} {bl}/{el}/{ml} takes the fast path");
        assert_eq!(format!("{:?}",berlin_run(&inp,30_000_000)),format!("{:?}",revm_run(&inp,30_000_000)),"{kind} {bl}/{el}/{ml}");
    }
}
/// A modulus of `len` bytes (at least 9) that is q·2^t with q odd: q of 1 to len bytes (1 and 3 included), t from 0
/// to what fits, the tie at t = 64 and the largest t (3·2^(8·len-2)) included; sometimes a power of two or a
/// value with high zero bytes. Most take the wide even path (t > 64, q > 1); the others must go to revm.
fn wide_modulus(r:&mut Rng,len:usize)->Vec<u8>{
    let bits=8*len;
    let qb:Vec<u8>=match r.below(6){0=>vec![1],1=>vec![3],2=>{let n=1+r.below(8) as usize;bytes(r,n)},_=>{let n=1+r.below(len as u64) as usize;bytes(r,n)}};
    let mut q=limbs(&qb);q[0]|=1;let qbits=64*q.len()-q[q.len()-1].leading_zeros() as usize;
    if qbits>bits{return modulus(r,len)}
    let room=bits-qbits;let t=match r.below(5){0=>room,1=>64.min(room),2=>65.min(room),_=>r.below(room as u64+1) as usize};
    let mut words=vec![0u64;t/64];let r0=t%64;let mut carry=0u64;
    for &x in &q{words.push((x<<r0)|carry);carry=if r0==0{0}else{x>>(64-r0)};}words.push(carry);
    let b=be(&words);let mut m=vec![0u8;len-b.len().min(len)];m.extend_from_slice(&b[b.len().saturating_sub(len)..]);m
}
/// The wide even path returns what revm returns, Ok (gas and bytes) or Err, on random moduli q·2^t of 9 to 160
/// bytes with random bases (empty, zero, longer than the modulus) and exponents (empty, zero, short, all ones
/// up to 256 bytes), inputs cut short and gas limits below the cost; positive control: it ran in more than a
/// third of the cases.
#[test] fn the_wide_even_path_returns_what_revm_returns(){
    let mut r=Rng(0x7e3);let (mut wide,mut cases)=(0u32,0u32);
    for _ in 0..20_000{
        let ml=9+r.below(152) as usize;let m=wide_modulus(&mut r,ml);
        let b=match r.below(5){0=>vec![],1=>vec![0;r.below(20) as usize],2=>{let n=ml+r.below(40) as usize;bytes(&mut r,n)},_=>{let n=r.below(ml as u64+1) as usize;bytes(&mut r,n)}};
        let e=match r.below(6){0=>vec![],1=>vec![0;r.below(40) as usize],2=>vec![r.below(4) as u8],3=>vec![0xff;1+r.below(256) as usize],_=>{let n=1+r.below(40) as usize;bytes(&mut r,n)}};
        let mut inp=input(&b,&e,&m);
        if r.below(12)==0{let cut=r.below(inp.len() as u64+1) as usize;inp.truncate(cut);}
        let gas=[200,5_000,1_000_000,30_000_000,u64::MAX][r.below(5) as usize];
        let bytes=Bytes::from(inp);
        let (mine,theirs)=(berlin_run(&bytes,gas),revm_run(&bytes,gas));
        assert_eq!(format!("{mine:?}"),format!("{theirs:?}"),"input {bytes} gas {gas}");
        if wide_even_split(&m).is_some()&&fast(&bytes,gas).is_some(){wide+=1}cases+=1;
    }
    assert!(wide>cases/3,"positive control: the wide even path ran in {wide} of {cases} cases");
}
/// inv_pow2(q, n) is q^-1 mod 2^(64n): q·x = 1 mod 2^(64n), for odd q of 1 to 12 words and n of 1 to 40.
#[test] fn inv_pow2_inverts_mod_a_power_of_two(){
    let mut r=Rng(0x1b7);
    for _ in 0..3_000{
        let qn=1+r.below(12) as usize;let mut q:Vec<u64>=(0..qn).map(|_|r.next()).collect();q[0]|=1;if r.below(4)==0{q.truncate(1);q[0]=[1,3,u64::MAX][r.below(3) as usize]}
        let n=1+r.below(40) as usize;let x=inv_pow2(&q,n);assert_eq!(x.len(),n);
        let p=mul_trunc(&q,&x,n);assert!(p[0]==1&&p[1..].iter().all(|w|*w==0),"q {q:x?} n {n}: q·x = {p:x?}");
    }
}
/// The slowest MODEXP shapes of the 2026-10-27 sweep (journal_measure's jm_modexp_sweep), each against revm:
/// 3·2^(8m-2) and an even modulus whose low half is zero, with exponents of two bits and of all ones.
#[test] fn the_swept_wide_shapes_match_revm(){
    for (ml,kind,e,wide) in [(256usize,"evenmax",vec![3u8],true),(512,"evenmax",vec![3],true),(1024,"evenmax",vec![3],true),(128,"evenmax",vec![0xff;64],true),
        (16,"evenmax",vec![0xff;256],true),(256,"evenhalf",vec![3],true),(1024,"evenhalf",vec![3],true),(16,"evenhalf",vec![0xff;256],false),(64,"evenhalf",vec![0x03,0xff],true)]{
        let fill=|n:usize,s:u8|->Vec<u8>{(0..n).map(|i|(i as u8).wrapping_mul(37).wrapping_add(s)).collect()};
        let mut b=fill(ml,11);b[0]|=0x80;
        let m=if kind=="evenmax"{let mut m=vec![0u8;ml];m[0]=0xc0;m}else{let mut m=fill(ml,101);m[0]|=0x80;let h=ml/2;for x in &mut m[ml-h..]{*x=0}m[ml-h-1]|=1;m};
        let inp=Bytes::from(input(&b,&e,&m));
        assert_eq!(wide_even_split(&m).is_some()&&fast(&inp,30_000_000).is_some(),wide,"{kind} m{ml}: the wide path is taken iff t > 64");
        assert_eq!(format!("{:?}",berlin_run(&inp,30_000_000)),format!("{:?}",revm_run(&inp,30_000_000)),"{kind} m{ml}");
    }
}
/// Pocol's EVM runs this precompile at address 5, and its other precompiles are revm's Cancun set.
#[test] fn the_engine_runs_this_modexp_at_address_5(){
    use revm::{ContextPrecompile,precompile::{Precompile,PrecompileSpecId,Precompiles}};
    let db=revm::InMemoryDB::default();
    let block=crate::native::NativeBlockContext{height:U256::ZERO,timestamp:U256::ZERO,gas_limit:U256::from(30_000_000u64),base_fee:U256::ZERO,prevrandao:Default::default()};
    let evm=crate::native::build_engine(&db,None,Default::default(),&block,1,None);
    let p=(evm.handler.pre_execution.load_precompiles)();
    let cancun=Precompiles::new(PrecompileSpecId::CANCUN);
    let mut ours:Vec<_>=p.addresses().copied().collect();ours.sort();let mut theirs:Vec<_>=cancun.addresses().copied().collect();theirs.sort();
    assert_eq!(ours,theirs,"the same addresses as revm's Cancun precompiles");
    let mut p=p;
    match p.to_mut().get(&u64_to_address(5)){Some(ContextPrecompile::Ordinary(Precompile::Standard(f)))=>assert!(*f as *const ()==berlin_run as *const (),"address 5 is crate::modexp::berlin_run"),
        _=>panic!("address 5 is not a standard precompile")}
}
