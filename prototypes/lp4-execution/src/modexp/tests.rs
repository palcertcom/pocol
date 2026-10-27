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
