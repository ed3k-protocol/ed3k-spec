// Names (name.md §2, §3.2 and §4.2). Prints the section of ../vectors-v1.md as markdown.
use super::*;

fn b32(d: &[u8]) -> String {
    const A: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
    let (mut o, mut acc, mut bits) = (String::new(), 0u32, 0);
    for &b in d { acc = (acc << 8) | b as u32; bits += 8; while bits >= 5 { bits -= 5; o.push(A[((acc >> bits) & 31) as usize] as char); } }
    if bits > 0 { o.push(A[((acc << (5 - bits)) & 31) as usize] as char); }
    o
}

pub fn run(pk1: &[u8], seed1: &[u8], pk2: &[u8], seed2: &[u8]) {
    let sign = |seed: &[u8], m: &[u8]| { let sk = SigningKey::from_bytes(seed.try_into().unwrap()); let s = sk.sign(m); sk.verifying_key().verify_strict(m, &s).unwrap(); s.to_bytes().to_vec() };
    let t: u64 = 1791849600;
    assert_eq!(b32(b"blog"), "mjwg6zy");

    // targets (§3.2, §4.2)
    let apex = h("ed3k v1 name", pk1)[..16].to_vec();
    let blog = h("ed3k v1 name", &cat(&[pk1, b"blog"]))[..16].to_vec();
    let shop = h("ed3k v1 name", &cat(&[pk1, b"shop.blog"]))[..16].to_vec();
    let alice = h("ed3k v1 name claim", b"alice")[..16].to_vec();

    // delegation (§2): root = sample key, signing key = second key
    let (issued, expires, scope) = (t, t + 30 * 86400, 1u64);
    let dpre = cat(&[b"ed3k v1 delegation", pk1, pk2, &issued.to_be_bytes(), &expires.to_be_bytes(), &scope.to_be_bytes()]);
    let dsig = sign(seed1, &dpre);
    let deleg = cat(&[&fb(1, pk2), &fv(2, issued), &fv(3, expires), &fv(4, scope), &fm(5, &sigmsg(&dsig))]);

    // NAME record for "blog", signed by the signing key
    let link = format!("ed3k:set?key={}&id=mjwg6zy", b32(pk1));
    let name = cat(&[&fb(1, pk1), &fb(2, b"blog"), &fv(3, 1), &fb(4, link.as_bytes()), &fm(6, &deleg)]);
    let nbody = cat(&[&fv(1, t + 7 * 86400), &fm(12, &name)]);
    let npre = cat(&[b"ed3k v1 record", &[14u8], &blog, &nbody]);
    let nsig = sign(seed2, &npre);
    let nsigned = cat(&[&fb(1, &nbody), &fm(3, &sigmsg(&nsig))]);
    assert!(nsigned.len() <= 1024);

    // CLAIM record: the second key calls the sample key's zone "alice"
    let claim = cat(&[&fb(1, b"alice"), &fb(2, pk1)]);
    let cbody = cat(&[&fv(1, t + 86400), &fm(13, &claim)]);
    let cpre = cat(&[b"ed3k v1 record", &[15u8], &alice, &cbody]);
    let csig = sign(seed2, &cpre);
    let csigned = cat(&[&fb(1, &cbody), &fm(3, &sigmsg(&csig))]);

    println!("\n## Names (name.md §2, §3.2 and §4.2)\n");
    println!("The zone has the sample key as its root key and the second key as its signing key. The sample");
    println!("time is `issued` of the delegation.\n");
    println!("**Keys in links:**\n\n```");
    println!("sample key  {}\nsecond key  {}", b32(pk1), b32(pk2));
    println!("```\n\n**Targets:**\n\n```");
    println!("apex          = H(\"ed3k v1 name\", key)[0..16) = {}", hx(&apex));
    println!("\"blog\"        = H(\"ed3k v1 name\", key ‖ \"blog\")[0..16) = {}", hx(&blog));
    println!("\"shop.blog\"   = H(\"ed3k v1 name\", key ‖ \"shop.blog\")[0..16) = {}", hx(&shop));
    println!("claim \"alice\" = H(\"ed3k v1 name claim\", \"alice\")[0..16) = {}", hx(&alice));
    println!("```\n\n**Delegation**, `issued` {issued}, `expires` {expires}, `scope` 1:\n\n```");
    wrap("signed by the root key", &dpre);
    println!("signature {}", hx(&dsig));
    wrap("Delegation", &deleg);
    println!("```\n\n**`NAME` record** for `blog`, `seq` 1, the link `{link}`,");
    println!("`expires` {}, signed by the signing key under the target of `blog`:\n\n```", t + 7 * 86400);
    wrap("body", &nbody);
    println!("signature {}", hx(&nsig));
    wrap("Signed", &nsigned);
    println!("```\n\n**`CLAIM` record** by the second key: `alice` is the zone of the sample key, `expires` {}:\n\n```", t + 86400);
    wrap("body", &cbody);
    println!("signature {}", hx(&csig));
    wrap("Signed", &csigned);
    println!("```");
}
