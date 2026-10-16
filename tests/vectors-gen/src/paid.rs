// Paid subscriptions (subscription.md): scopes, a season chain and its tip, offer, pass, proof,
// key request, sealed segment, member and hidden head, topic target, share report.
// Prints the section of ../vectors-v1.md as markdown.
use super::*;

fn w(h: &str, ind: usize) -> String { h.as_bytes().chunks(96).map(|c| std::str::from_utf8(c).unwrap().to_string()).collect::<Vec<_>>().join(&format!("\n{}", " ".repeat(ind))) }
fn signed(body: &[u8], sig: &[u8]) -> Vec<u8> { cat(&[&fb(1, body), &fm(2, &sigmsg(sig))]) }
// K(e) of a season of `scope` from its seed K(last); every step is bound to the scope (§4.1)
fn step(scope: &[u8], k: &[u8; 32]) -> [u8; 32] { h("ed3k v1 paid chain", &cat(&[scope, k])) }
fn chain_key(scope: &[u8], seed: &[u8; 32], last: u64, e: u64) -> [u8; 32] { let mut k = *seed; for _ in e..last { k = step(scope, &k) } k }
fn anchor_of(scope: &[u8], k: &[u8; 32]) -> [u8; 32] { h("ed3k v1 paid anchor", &cat(&[scope, k])) }

pub fn run(pk1: &[u8], seed1: &[u8], node_id: &[u8], pk2: &[u8], sample: &[u8], dir: &Option<String>) {
    let dump = |name: &str, d: &[u8]| if let Some(dir) = dir { std::fs::write(format!("{dir}/{name}"), d).unwrap() };
    let sk1 = SigningKey::from_bytes(seed1.try_into().unwrap());
    let sign = |sk: &SigningKey, m: &[u8]| { let s = sk.sign(m); sk.verifying_key().verify_strict(m, &s).unwrap(); s.to_bytes().to_vec() };
    let nonce = unhex("000102030405060708090a0b");
    let time = 1791849600u64; let day = 20739u64;

    // scopes
    let scope = |kind: u8, id: &[u8]| h("ed3k v1 paid scope", &cat(&[&[kind], pk1, id]))[..16].to_vec();
    let (s1, s2, s3) = (scope(1, b"sid1"), scope(2, b"sid1"), scope(3, b""));

    // season A: epochs 270..299; a leak in epoch 284 starts season B at 285 = that epoch + 1 (§4.5).
    // The sample time is in epoch 285, the first of B: A has ended, and its tip is K(284)
    let (t0, period) = (1767225600u64, 86400u64);
    let epoch = (time - t0) / period; assert_eq!(epoch, 285);
    let seed_a: [u8; 32] = core::array::from_fn(|i| 0x1f ^ i as u8);
    let (base_a, len_a) = (270u64, 30u64); let last_a = base_a + len_a - 1;
    let k285a = chain_key(&s1, &seed_a, last_a, 285);
    let k284 = step(&s1, &k285a); assert_eq!(k284, chain_key(&s1, &seed_a, last_a, 284));
    let k270 = chain_key(&s1, &seed_a, last_a, base_a);
    let anchor_a = anchor_of(&s1, &k270);
    let seed_b: [u8; 32] = core::array::from_fn(|i| 0x4e ^ i as u8);
    let (base_b, len_b) = (285u64, 30u64); let last_b = base_b + len_b - 1;
    assert_eq!(base_b, 284 + 1); assert!(base_b <= epoch && base_b == base_a + 15);
    let k285 = chain_key(&s1, &seed_b, last_b, 285);
    let anchor_b = anchor_of(&s1, &chain_key(&s1, &seed_b, last_b, base_b));
    let tip_a = chain_key(&s1, &seed_a, last_a, base_b - 1); assert_eq!(tip_a, k284);
    // a reader's check (§4.1): hash down to base, then the anchor step
    let verify = |k: &[u8; 32], e: u64, base: u64, anchor: &[u8; 32]| { let mut x = *k; for _ in base..e { x = step(&s1, &x) } anchor_of(&s1, &x) == *anchor };
    assert!(verify(&tip_a, base_b - 1, base_a, &anchor_a) && verify(&k285a, 285, base_a, &anchor_a));
    assert!(verify(&k285, 285, base_b, &anchor_b) && !verify(&k285, 285, base_a, &anchor_a));

    let chain = |base: u64, len: u64, anchor: &[u8]| cat(&[&fb(1, &s1), &fv(2, base), &fv(3, len), &fb(4, anchor)]);
    let chain_a = chain(base_a, len_a, &anchor_a); let chain_b = chain(base_b, len_b, &anchor_b);
    let cpre_a = cat(&[b"ed3k v1 paid season", &chain_a]); let csig_a = sign(&sk1, &cpre_a);
    let cpre_b = cat(&[b"ed3k v1 paid season", &chain_b]); let csig_b = sign(&sk1, &cpre_b);
    let schain_a = signed(&chain_a, &csig_a); let schain_b = signed(&chain_b, &csig_b);
    dump("paid_chain.bin", &chain_a); dump("paid_schain.bin", &schain_a);

    // offer
    let offer = cat(&[&fb(1, &s1), &fv(2, time), &fv(3, t0), &fv(4, period), &fm(5, &schain_b),
        &fb(7, b"4.99 EUR / month"), &fb(8, b"https://example.com/pay"), &fb(9, b"https://example.com/pass"),
        &fb(12, b"https://example.com/report"), &fv(13, 1)]);
    let opre = cat(&[b"ed3k v1 paid offer", &offer]); let osig = sign(&sk1, &opre);
    let soffer = signed(&offer, &osig);
    dump("paid_offer.bin", &offer); dump("paid_soffer.bin", &soffer);

    // holder key and pass
    let hseed: Vec<u8> = (0..32u8).map(|i| 0x6b ^ i).collect();
    let hsk = SigningKey::from_bytes(hseed.as_slice().try_into().unwrap()); let holder = hsk.verifying_key().to_bytes().to_vec();
    let until = time + 30 * 86400;
    let pass = cat(&[&fb(1, &s1), &fb(2, &holder), &fv(3, until), &fv(5, time)]);
    let ppre = cat(&[b"ed3k v1 paid pass", &pass]); let psig = sign(&sk1, &ppre);
    let spass = cat(&[&fb(1, &pass), &fb(2, pk1), &fm(3, &sigmsg(&psig))]);
    dump("paid_pass.bin", &pass); dump("paid_spass.bin", &spass);

    // proof: viewer node = node of the second key, receiver = sample node
    let viewer = h("ed3k v1 node id", pk2)[..16].to_vec();
    let minute = time / 60;
    let proof_of = |sender: &[u8], m: u64| { let pre = cat(&[b"ed3k v1 paid proof", &s1, sender, node_id, &m.to_be_bytes()]); (pre.len(), sign(&hsk, &pre)) };
    let (prelen, prsig) = proof_of(&viewer, minute);
    let pproof = cat(&[&fv(1, minute), &fb(2, &prsig)]);
    dump("paid_proof.bin", &pproof);

    // key request and answer
    let kreq = cat(&[&fb(1, &s1), &fm(2, &spass), &fm(3, &pproof), &fv(5, 1)]);
    dump("paid_keyreq.bin", &kreq);
    // the answer: the key of the current epoch, the offer asked for, the tip of the ended season A
    let ekey = |e: u64, k: &[u8]| cat(&[&fv(1, e), &fb(2, k)]);
    let kans = cat(&[&fm(1, &ekey(285, &k285)), &fm(3, &soffer), &fm(5, &ekey(base_b - 1, &tip_a)), &fm(6, &schain_b), &fm(6, &schain_a)]);
    dump("paid_keyans.bin", &kans);

    // sealed segment
    let k_seg = h("ed3k v1 paid live", &cat(&[&s1, &k285]));
    let stored = cat(&[&nonce, &chacha(&k_seg, &nonce, sample)]);
    let sroot = blake3::hash(&stored).as_bytes().to_vec();
    let stream = h("ed3k v1 live", &cat(&[pk1, b"sid1"]))[..16].to_vec();
    let seg = cat(&[&fv(1, time), &fv(2, 7), &fb(3, &sroot), &fv(4, stored.len() as u64), &fv(5, 4000), &fv(10, 285)]);
    let lpre = cat(&[b"ed3k v1 live segment", &stream, &seg]); let lsig = sign(&sk1, &lpre);
    dump("paid_seg.bin", &seg);

    // sealed member and hidden head: the hidden set, scope(3), has a season B of its own with the
    // seed and bounds of B; its chain is bound to scope(3), so its keys differ from those of scope(1)
    let k285h = chain_key(&s3, &seed_b, last_b, 285); assert_ne!(k285h, k285);
    let k_data = h("ed3k v1 paid data", &cat(&[&s3, &k285h]));
    let sealed_m = cat(&[&nonce, &chacha(&k_data, &nonce, sample)]);
    let k_head = h("ed3k v1 paid head", &cat(&[&s3, &k285h]));
    let root = blake3::hash(sample).as_bytes().to_vec();
    let head = cat(&[&fb(1, b"sid1"), &fv(2, 1), &fb(3, &root), &fv(4, 16)]);
    let hdata = cat(&[&nonce, &chacha(&k_head, &nonce, &head)]);
    let sealed_rec = cat(&[&fv(1, 1), &fb(2, &hdata), &fv(3, 285)]);
    dump("paid_sealed.bin", &sealed_rec);

    // topic target of the set scope
    let target = h("ed3k v1 paid topic", &cat(&[&s2, &day.to_be_bytes()]))[..16].to_vec();

    // share report: two devices seen, offer allows 1
    let second: Vec<u8> = (0..16u8).map(|i| 0x2c ^ i).collect();
    let (_, prsig2) = proof_of(&second, minute + 3);
    let sighting = |sender: &[u8], m: u64, sig: &[u8]| cat(&[&fb(1, sender), &fm(2, &cat(&[&fv(1, m), &fb(2, sig)]))]);
    let report = cat(&[&fb(1, &s1), &fm(2, &spass), &fb(3, node_id), &fm(4, &sighting(&viewer, minute, &prsig)), &fm(4, &sighting(&second, minute + 3, &prsig2))]);
    dump("paid_report.bin", &report);

    println!("\n==== markdown of the paid subscription batch ====");
    println!("\n## Paid subscriptions (subscription.md §2 to §8)\n
The publisher has the sample key; the channel and the set have the `id` `sid1`. `t0` is
1767225600 (2026-01-01 00:00 UTC) and `period` 86400, so the sample time lies in epoch 285.
The holder key is made from the seed byte *i* = `0x6b XOR i`; the viewer's node is the node of
the second key, the receiver the node of the sample key. Messages are canonical (§0).

**Scopes** (§2)

```
scope(1, key, \"sid1\") = {}      live channel
scope(2, key, \"sid1\") = {}      set with an open manifest
scope(3, key, \"\")     = {}      hidden set
```

**Season and keys** (§4.1, §4.2). Every chain step and the anchor are bound to `scope(1)`.
Season A covers epochs 270 to 299; its seed `K(299)` is 32 bytes, byte *i* = `0x1f XOR i`. A
leak in epoch 284 starts season B at 285, that epoch + 1 (§4.5); its seed `K(314)` is byte
*i* = `0x4e XOR i`. Season A is so abandoned after 284, and its tip is `K(284)`. At the sample
time, epoch 285, B is the current season and A has ended.

```
K(299) of A = {}
K(285) of A = {}
K(284) of A = H(\"ed3k v1 paid chain\", scope(1) ‖ K(285) of A) = {}
K(270) of A = {}
anchor A    = H(\"ed3k v1 paid anchor\", scope(1) ‖ K(270)) = {}
tip of A    = K(284) of A
anchor B    = {}
K(285) of B = {}
```

```
Chain A     = {{ scope = scope(1), base = 270, length = 30, anchor A }}
            = {}
              ({} bytes)
signed      = \"ed3k v1 paid season\" ‖ Chain A      ({} bytes)
signature   = {}
              {}
SignedChain = {}
              ({} bytes)
```

**Offer** (§3). `price` \"4.99 EUR / month\", `pay_url` `https://example.com/pay`,
`pass_url` `https://example.com/pass`, `report_url` `https://example.com/report`,
`devices` 1, `issued` the sample time, `chain` the SignedChain of B, the current season.

```
Offer       = {}
              ({} bytes)
signature   = {}
              {}
SignedOffer = {} bytes
```

**Pass** (§5) for `scope(1)`, signed by the publisher, `until` = 1794441600 (30 days later):

```
holder key  = {}
Pass        = {}
              ({} bytes)
signature   = {}
              {}
SignedPass  = {}
              ({} bytes)
```

**Proof** (§5.2), minute {}:

```
viewer node = {}
signed      = \"ed3k v1 paid proof\" ‖ scope(1) ‖ viewer node ‖ {} ‖ {}      ({} bytes)
signature   = {}
              {}
PassProof   = {}
              ({} bytes)
```

**Key exchange** (§6.1). The request shows the pass, holds no key and asks for the offer. The
answer gives `K(285)` of season B, the key of the current epoch, the offer, the tip of the
ended season A, and both `SignedChain`s:

```
KeyRequest  = {} bytes
KeyAnswer   = {}
              ({} bytes)
```

**Sealed segment** (§7.1). The sample file, sealed with the sample nonce under `K(285)` of B:

```
k_seg       = H(\"ed3k v1 paid live\", scope(1) ‖ K(285)) = {}
stored      = {}      ({} bytes)
root        = {}
LiveSegment = {{ start = 1791849600, seq = 7, root, size = 28, ms = 4000, epoch = 285 }}
            = {}
              ({} bytes)
signed      = \"ed3k v1 live segment\" ‖ stream ‖ LiveSegment      ({} bytes)
signature   = {}
              {}
```

**Sealed member and hidden head** (§8.1, §8.3). The head is that of the hidden set section.
The hidden set, `scope(3)`, has a season with the seed and the epochs of B; its chain is bound
to `scope(3)`, so its keys are not those of `scope(1)`:

```
K(285)      = {}      season B of scope(3)
k_data      = H(\"ed3k v1 paid data\", scope(3) ‖ K(285)) = {}
sealed file = {}      ({} bytes)
k_head      = H(\"ed3k v1 paid head\", scope(3) ‖ K(285)) = {}
Sealed      = {{ seq = 1, data = nonce ‖ ChaCha20(k_head, nonce, Head), epoch = 285 }}
            = {}
              ({} bytes)
```

**Topic of the set** (§8.4), sample day:

```
target      = H(\"ed3k v1 paid topic\", scope(2) ‖ 0000000000005103)[0..16) = {}
```

**Share report** (§5.3). The holder key shows the pass from the viewer's node at the sample
minute and from a node with ID byte *i* = `0x2c XOR i` three minutes later; `devices` is 1:

```
ShareReport = {}
              ({} bytes)
```",
        hx(&s1), hx(&s2), hx(&s3),
        hx(&seed_a), hx(&k285a), hx(&k284), hx(&k270), hx(&anchor_a), hx(&anchor_b), hx(&k285),
        w(&hx(&chain_a), 14), chain_a.len(), cpre_a.len(), hx(&csig_a[..32]), hx(&csig_a[32..]), w(&hx(&schain_a), 14), schain_a.len(),
        w(&hx(&offer), 14), offer.len(), hx(&osig[..32]), hx(&osig[32..]), soffer.len(),
        hx(&holder), w(&hx(&pass), 14), pass.len(), hx(&psig[..32]), hx(&psig[32..]), w(&hx(&spass), 14), spass.len(),
        minute, hx(&viewer), hx(node_id), hx(&minute.to_be_bytes()), prelen, hx(&prsig[..32]), hx(&prsig[32..]), w(&hx(&pproof), 14), pproof.len(),
        kreq.len(), w(&hx(&kans), 14), kans.len(),
        hx(&k_seg), hx(&stored), stored.len(), hx(&sroot), w(&hx(&seg), 14), seg.len(), lpre.len(), hx(&lsig[..32]), hx(&lsig[32..]),
        hx(&k285h), hx(&k_data), hx(&sealed_m), sealed_m.len(), hx(&k_head), w(&hx(&sealed_rec), 14), sealed_rec.len(),
        hx(&target),
        w(&hx(&report), 14), report.len());
}
