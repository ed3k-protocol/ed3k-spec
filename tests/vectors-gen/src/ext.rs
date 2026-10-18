// Second batch: bootstrap net_tag, kad datagrams, key puzzle, Usenet, hoster URLs, version 2
// and hybrid torrents, HTTP hash body. Prints the sections of ../vectors-v1.md as markdown.
// `PUZZLE_SEARCH=1` searches the lowest valid nonce again (about a minute on 12 threads).
use super::*;
use chacha20poly1305::aead::Payload;
use idna::uts46::{AsciiDenyList, DnsLength, Hyphens, Uts46};
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) fn w(h: &str, ind: usize) -> String { h.as_bytes().chunks(96).map(|c| std::str::from_utf8(c).unwrap().to_string()).collect::<Vec<_>>().join(&format!("\n{}", " ".repeat(ind))) }
fn seal(key: &[u8], nonce: &[u8], aad: &[u8], msg: &[u8]) -> Vec<u8> { ChaCha20Poly1305::new(key.into()).encrypt(nonce.into(), Payload { msg, aad }).unwrap() }
pub(crate) fn conv(seed: &[u8]) -> ([u8; 32], Vec<u8>) {
    let sk = SigningKey::from_bytes(seed.try_into().unwrap());
    let mut p = sha2::Sha512::digest(seed)[..32].to_vec(); p[0] &= 248; p[31] &= 127; p[31] |= 64;
    (sk.verifying_key().to_montgomery().0, p)
}

pub fn run(pk1: &[u8], seed1: &[u8], pk2: &[u8], seed2: &[u8], node_id: &[u8], sample: &[u8]) {
    println!("\n==== markdown of the second batch ====");
    net_tag();
    kad(pk1, seed1, pk2, seed2, node_id, sample);
    puzzle(pk1);
    usenet(sample);
    hoster();
    torrents(sample);
    hash_body();
}

fn net_tag() {
    let net_id = &h("ed3k v1 net", b"ed3k test")[..16];
    println!("\n## Node list (bootstrap.md §2)\n\n```\nnet_tag(\"ed3k test\") = H(\"ed3k v1 nodes net\", net_id)[0..16) = {}\n```", hx(&h("ed3k v1 nodes net", net_id)[..16]));
}

fn kad(pk1: &[u8], seed1: &[u8], pk2: &[u8], seed2: &[u8], node_id: &[u8], sample: &[u8]) {
    // responder R = sample key, requester I = second key at 21.75.31.124:4672
    let (xr_pub, xr_priv) = conv(seed1); let (xi_pub, xi_priv) = conv(seed2);
    let ss = x25519(&xi_priv, &xr_pub); assert_eq!(ss, x25519(&xr_priv, &xi_pub));
    let mut kk = [0u8; 64]; blake3::Hasher::new_derive_key("ed3k v1 kad dgram").update(&cat(&[&ss, pk2, pk1])).finalize_xof().fill(&mut kk);
    let (k_req, k_ans) = kk.split_at(32);
    let secret: Vec<u8> = (0..32u8).map(|i| 0xc3 ^ i).collect();
    let addr = [21u8, 75, 31, 124, 0x12, 0x40];
    let cookie = blake3::keyed_hash(secret.as_slice().try_into().unwrap(), &addr).as_bytes()[..8].to_vec();
    let obf = &node_id[..12]; let txid = unhex("0a0b0c0d"); let magic = [0x65u8, 0x33, 0x6b, 0x02];
    let dg = |dir: u8, salt: &[u8], pad: &[u8], typ: u8, payload: &[u8]| {
        let plain = cat(&[&magic, &[pad.len() as u8], pad, &[typ], &txid, payload]);
        cat(&[salt, &mask(&h("ed3k v1 obfs", &cat(&[obf, &[dir], salt])), &plain, usize::MAX)])
    };
    let pad: Vec<u8> = (0..52u8).collect();
    let init = dg(0, &unhex("5152535455565758"), &pad, 48, &[]); assert_eq!(init.len(), 70);
    let chal = dg(1, &unhex("6162636465666768"), &[], 49, &cat(&[&cookie, pk1])); assert_eq!(chal.len(), 58);
    let target = &blake3::hash(sample).as_bytes()[..16].to_vec();
    let req_body = cat(&[&fb(1, target), &fv(2, 1)]);
    let salt_q = unhex("7172737475767778");
    let plain_q = cat(&[&[0u8], &[3 << 1], &req_body]);
    let ct_q = seal(k_req, &cat(&[&salt_q, &txid]), &cat(&[&[50u8], &txid, pk2, &cookie]), &plain_q);
    let payload_q = cat(&[pk2, &cookie, &ct_q]);
    let req = dg(0, &salt_q, &[], 50, &payload_q);
    let req_hash = h("ed3k v1 kad req", &cat(&[&[50u8], &txid, &payload_q]))[..8].to_vec();
    let id2 = &h("ed3k v1 node id", pk2)[..16];
    let ans_body = cat(&[&fb(1, &cat(&[id2, &[65, 23, 51, 170, 0x12, 0x40]])), &fv(3, 1)]);
    let salt_a = unhex("8182838485868788");
    let plain_a = cat(&[&[1u8], &[4 << 1], &ans_body]);
    let ct_a = seal(k_ans, &cat(&[&salt_a, &txid]), &cat(&[&[51u8], &txid, &req_hash]), &plain_a);
    let ans = dg(1, &salt_a, &[], 51, &ct_a);
    if let Some(d) = std::env::args().nth(2) { std::fs::write(format!("{d}/kadreq.bin"), &req_body).unwrap(); std::fs::write(format!("{d}/kadans.bin"), &ans_body).unwrap(); }
    println!("\n## Kad datagrams (kad-datagram.md §2 and §3)\n
The responder has the sample key. The requester has the second key and sends from
`21.75.31.124`, port 4672. `txid` is `0a0b0c0d`. All four datagrams are masked with the
responder's `obf_id`, in the public network.

```
second key, converted (§6.4): x_public  = {}
                              x_private = {}
ss            = {}
k_req ‖ k_ans = H(\"ed3k v1 kad dgram\", ss ‖ second key ‖ sample key), 64 bytes
k_req         = {}
k_ans         = {}

secret_e      = 32 bytes, byte i = 0xc3 XOR i
cookie        = BLAKE3_keyed(secret_e, 154b1f7c ‖ 1240)[0..8) = {}

INIT, dir 0x00, salt 5152535455565758, pad_len 52 (pad byte i = i)      (70 bytes)
  datagram    = {}

CHALLENGE, dir 0x01, salt 6162636465666768, pad_len 0, payload = cookie ‖ sample key      (58 bytes)
  datagram    = {}

REQUEST, dir 0x00, salt 7172737475767778, pad_len 0
  KadRequest  = {{ target = root[0..16) of the sample file, want = 1 }} = {}
  plaintext   = nf 00 ‖ tag 06 ‖ KadRequest      ({} bytes)
  nonce       = 71727374757677780a0b0c0d
  aad         = 32 ‖ txid ‖ second key ‖ cookie      (45 bytes)
  sealed      = {}
  payload     = second key ‖ cookie ‖ sealed      ({} bytes)
  req_hash    = H(\"ed3k v1 kad req\", 32 ‖ txid ‖ payload)[0..8) = {}
  datagram    = {}

ANSWER, dir 0x01, salt 8182838485868788, pad_len 0
  KadAnswer   = {{ nodes4 = node ID of the second key ‖ 65.23.51.170 port 4672, responsible = true }}
              = {}
  plaintext   = nf 01 ‖ tag 08 ‖ KadAnswer      ({} bytes)
  nonce       = 81828384858687880a0b0c0d
  aad         = 33 ‖ txid ‖ req_hash      (13 bytes)
  sealed      = {}
  datagram    = {}
```",
        hx(&xi_pub), hx(&xi_priv), hx(&ss), hx(k_req), hx(k_ans), hx(&cookie), w(&hx(&init), 16), w(&hx(&chal), 16),
        hx(&req_body), plain_q.len(), w(&hx(&ct_q), 16), payload_q.len(), hx(&req_hash), w(&hx(&req), 16),
        hx(&ans_body), plain_a.len(), w(&hx(&ct_a), 16), w(&hx(&ans), 16));
}

fn argon(pwd: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, argon2::Params::new(65536, 1, 1, Some(32)).unwrap()).hash_password_into(pwd, b"ed3k v1 puzzle", &mut out).unwrap();
    out
}
fn zero_bits(o: &[u8]) -> u32 { let mut n = 0; for b in o { if *b == 0 { n += 8 } else { n += b.leading_zeros(); break } } n }
const PUZZLE_NONCE: u64 = 1330; // lowest valid nonce, found with PUZZLE_SEARCH=1
fn puzzle(pk1: &[u8]) {
    let epoch = (20739u64 * 86400 / 2_592_000) as u32;
    let pre = cat(&[pk1, &[4], &[124, 31, 75, 0, 0, 0], &epoch.to_be_bytes()]);
    let mut nonce = PUZZLE_NONCE;
    if std::env::var("PUZZLE_SEARCH").is_ok() {
        let best = AtomicU64::new(u64::MAX); let t = 12u64;
        std::thread::scope(|s| for i in 0..t { let (best, pre) = (&best, &pre); s.spawn(move || { let mut n = i; while n < best.load(Ordering::Relaxed) { if zero_bits(&argon(&cat(&[pre, &n.to_be_bytes()]))) >= 13 { best.fetch_min(n, Ordering::Relaxed); } n += t; } }); });
        nonce = best.load(Ordering::Relaxed); eprintln!("lowest valid nonce: {nonce}");
    }
    let o0 = argon(&cat(&[&pre, &0u64.to_be_bytes()])); let o = argon(&cat(&[&pre, &nonce.to_be_bytes()]));
    assert!(zero_bits(&o) >= 13, "set PUZZLE_SEARCH=1");
    println!("\n## Key puzzle (key-puzzle.md §2 and §3)\n
The sample key at `124.31.75.21`, on day 20739: epoch {epoch}.

```
password = public key ‖ 04 ‖ 7c1f4b000000 ‖ {} ‖ nonce      (51 bytes)
salt     = \"ed3k v1 puzzle\", t = 1, m = 65536 KiB, p = 1, 32 bytes out, Argon2id version 0x13

nonce 0000000000000000: out = {}      {} zero bits, not valid
nonce {}: out = {}      {} zero bits, valid
Hello.puzzle = {}
```

The second nonce is the lowest one, counted as a big-endian number from 0, that gives 13 zero
bits.", hx(&epoch.to_be_bytes()), hx(&o0), zero_bits(&o0), hx(&nonce.to_be_bytes()), hx(&o), zero_bits(&o), hx(&cat(&[&epoch.to_be_bytes(), &nonce.to_be_bytes()])));
}

fn usenet(sample: &[u8]) {
    let fid = |ids: &[&str]| { let mut d = vec![]; for m in ids { d.extend(m.as_bytes()); d.push(0) } h("ed3k v1 nzb file", &d) };
    let files: [(&str, &[u8], Vec<&str>); 2] = [("sample.txt", sample, vec!["p1.sample@vectors.invalid", "p2.sample@vectors.invalid"]), ("h\u{e9}llo.bin", b"hello", vec!["p1.hello@vectors.invalid"])];
    let mut v: Vec<([u8; 32], &str, &[u8], &Vec<&str>)> = files.iter().map(|(n, d, ids)| (fid(ids), *n, *d, ids)).collect();
    v.sort();
    let post = h("ed3k v1 nzb", &v.iter().flat_map(|f| f.0).collect::<Vec<u8>>());
    let list = cat(&[&unhex("6564336b6e7a6201"), &v.iter().flat_map(|f| fm(1, &cat(&[&fb(1, f.1.as_bytes()), &fv(2, f.2.len() as u64), &f.3.iter().flat_map(|m| fb(3, m.as_bytes())).collect::<Vec<u8>>()]))).collect::<Vec<u8>>()]);
    let lroot = blake3::hash(&list);
    let body = cat(&[&v.iter().flat_map(|f| fb(3, &cat(&[&fb(1, blake3::hash(f.2).as_bytes()), &fv(2, f.2.len() as u64), &fb(3, f.1.as_bytes())]))).collect::<Vec<u8>>(), &fm(10, &cat(&[&fb(1, &post), &fb(2, lroot.as_bytes()), &fv(3, list.len() as u64)]))]);
    let man = cat(&[&unhex("6564336b73657401"), &fb(1, &body)]);
    if let Some(d) = std::env::args().nth(2) { std::fs::write(format!("{d}/nzblist.bin"), &list[8..]).unwrap(); std::fs::write(format!("{d}/nzbset.bin"), &body).unwrap(); }
    println!("\n## Usenet post (usenet.md §1 to §3)\n
A post of two files. The articles of `sample.txt` (the sample file, 16 bytes) have the message
IDs `p1.sample@vectors.invalid` and `p2.sample@vectors.invalid`, parts 1 and 2; the one
article of the second file (the 5 ASCII bytes `hello`) has `p1.hello@vectors.invalid`. Its yEnc
header gives the name as the bytes `68 e9 6c 6c 6f 2e 62 69 6e`, which are not UTF-8: they are
read as ISO 8859-1 and written as `héllo.bin` (`68 c3 a9 6c 6c 6f 2e 62 69 6e`).

```
file id of sample.txt = H(\"ed3k v1 nzb file\", m₁ ‖ 00 ‖ m₂ ‖ 00) = {}
file id of héllo.bin  = H(\"ed3k v1 nzb file\", m₁ ‖ 00)          = {}
order of the files    = {}, then {}
post id               = H(\"ed3k v1 nzb\", f₁ ‖ f₂) = {}

article list = \"ed3knzb\" ‖ 01 ‖ ArticleList      ({} bytes)
             = {}
its root     = {}

manifest     = \"ed3kset\" ‖ 01 ‖ SetFile{{ body }}, FileSet = {{ files, usenet = {{ id, list, list_size }} }}      ({} bytes)
             = {}
its root     = {}
```",
        hx(&fid(&files[0].2)), hx(&fid(&files[1].2)), v[0].1, v[1].1, hx(&post), list.len(), w(&hx(&list), 15), lroot.to_hex(), man.len(), w(&hx(&man), 15), blake3::hash(&man).to_hex());
}

fn v6(a: std::net::Ipv6Addr) -> String {
    let s = a.segments(); let (mut bs, mut bl) = (0, 0); let mut i = 0;
    while i < 8 { let mut j = i; while j < 8 && s[j] == 0 { j += 1 } if j - i > bl { bs = i; bl = j - i } i = j.max(i + 1) }
    let g = |r: &[u16]| r.iter().map(|x| format!("{x:x}")).collect::<Vec<_>>().join(":");
    if bl < 2 { g(&s) } else { format!("{}::{}", g(&s[..bs]), g(&s[bs + bl..])) }
}
fn pass(x: &str) -> Option<String> {
    let (b, mut o, mut i) = (x.as_bytes(), String::new(), 0);
    let unres = |c: u8| c.is_ascii_alphanumeric() || b"-._~".contains(&c);
    while i < b.len() {
        let c = b[i];
        if c == b'%' {
            let hh = x.get(i + 1..i + 3).filter(|s| s.bytes().all(|d| d.is_ascii_hexdigit()))?;
            let v = u8::from_str_radix(hh, 16).ok()?;
            if unres(v) { o.push(v as char) } else { o += &format!("%{v:02X}") }
            i += 3; continue;
        }
        if unres(c) || b":/?#[]@!$&'()*+,;=".contains(&c) { o.push(c as char) } else { o += &format!("%{c:02X}") }
        i += 1;
    }
    Some(o)
}
// hoster.md §4, steps 1 to 8
pub fn canonical(link: &str) -> Option<String> {
    let (rest, frag) = link.split_once('#').unwrap_or((link, ""));
    let (rest, query) = rest.split_once('?').unwrap_or((rest, ""));
    let (scheme, rest) = rest.split_once(':')?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") { return None }
    let rest = rest.strip_prefix("//")?;
    let (auth, path) = match rest.find('/') { Some(i) => rest.split_at(i), None => (rest, "") };
    let auth = auth.rsplit_once('@').map_or(auth, |x| x.1);
    let host = if auth.starts_with('[') {
        let (inner, port) = auth[1..].split_once(']')?;
        if !(port.is_empty() || port.starts_with(':') && port[1..].bytes().all(|c| c.is_ascii_digit())) { return None }
        format!("[{}]", v6(inner.parse().ok()?))
    } else {
        let name = match auth.rsplit_once(':') { Some((n, p)) if p.bytes().all(|c| c.is_ascii_digit()) => n, _ => auth };
        if name.is_empty() { return None }
        let a = Uts46::new().to_ascii(name.as_bytes(), AsciiDenyList::STD3, Hyphens::Allow, DnsLength::Ignore).ok()?.into_owned();
        let a = a.strip_suffix('.').unwrap_or(&a);
        if a.is_empty() || a.len() > 253 || a.split('.').any(|l| l.is_empty() || l.len() > 63) { return None }
        let last = a.rsplit('.').next()?;
        if last.bytes().all(|c| c.is_ascii_digit()) || last.starts_with("0x") {
            let p: Vec<&str> = a.split('.').collect();
            if p.len() != 4 || p.iter().any(|n| n.is_empty() || !n.bytes().all(|c| c.is_ascii_digit()) || (n.len() > 1 && n.starts_with('0')) || n.parse::<u32>().map_or(true, |v| v > 255)) { return None }
            a.to_string()
        } else { let mut a = a; while let Some(r) = a.strip_prefix("www.") { a = r } a.to_string() }
    };
    let (p, q, f) = (pass(path)?, pass(query)?, pass(frag)?);
    Some(format!("{host}{}{}{}", if p.is_empty() { "/".into() } else { p }, if q.is_empty() { String::new() } else { format!("?{q}") }, if f.is_empty() { String::new() } else { format!("#{f}") }))
}
fn hoster() {
    let cases = ["http://example.com", "https://example.com/?#", "HTTPS://User:p@ss@WWW.Example.COM:8443/a/../B%7e%2f?x=1#Frag", "https://www.www.example.com:/", "http://www.www/",
        "https://example.com/100%25/%2541/%41?a+b=c d", "https://example.com/a b\"<é>{}", "https://example.com/p?a?b#c#d", "https://faß.example./ü", "https://Bücher.example/",
        "https://r3---sn-abc.example/v", "http://[2001:DB8:0:0:0:0:0:1]:8080/x", "http://[::ffff:1.2.3.4]/", "http://[2001:db8:0:1:0:0:0:1]/", "http://192.168.1.1:8080/f",
        "http://192.168.01.1/", "http://192.168.1/", "http://0x7f.0.0.1/", "http://example.com/%zz", "http://example.com/50%", "http://ex%61mple.com/", "http://[fe80::1%25eth0]/", "http:///path", "ftp://example.com/f", "http://exa_mple.com/"];
    println!("\n## Hoster links (hoster.md §4)\n\n| Link | `canonical` | `id` = H(\"ed3k v1 url\", canonical) |\n|---|---|---|");
    for c in cases { match canonical(c) { Some(k) => { assert_eq!(canonical(&format!("http://{k}")).as_deref(), Some(k.as_str())); println!("| `{c}` | `{k}` | `{}` |", hx(&h("ed3k v1 url", k.as_bytes()))) } None => println!("| `{c}` | invalid | none |") } }
}

fn torrents(sample: &[u8]) {
    let (a, b) = (sample, b"hello".as_slice());
    let s256 = |d: &[u8]| sha2::Sha256::digest(d).to_vec();
    let tree = cat(&[b"d5:a.txtd0:d6:lengthi16e11:pieces root32:", &s256(a), b"ee5:emptyd0:d6:lengthi0eee3:subd5:b.bind0:d6:lengthi5e11:pieces root32:", &s256(b), b"eeee"]);
    let v2 = cat(&[b"d9:file tree", &tree, b"12:meta versioni2e4:name8:ed3k vec12:piece lengthi16384ee"]);
    let mut p0 = a.to_vec(); p0.resize(16384, 0);
    let pieces = cat(&[&sha1::Sha1::digest(&p0), &sha1::Sha1::digest(b)]);
    let hy = cat(&[b"d9:file tree", &tree, b"5:filesld6:lengthi16e4:pathl5:a.txteed4:attr1:p6:lengthi16368e4:pathl4:.pad5:16368eed6:lengthi0e4:pathl5:emptyeed6:lengthi5e4:pathl3:sub5:b.bineee12:meta versioni2e4:name8:ed3k vec12:piece lengthi16384e6:pieces40:", &pieces, b"e"]);
    let man = |info: &[u8], v1: bool| {
        let m = |d: &[u8], n: &str, dir: u64| fb(3, &cat(&[&fb(1, blake3::hash(d).as_bytes()), &fv(2, d.len() as u64), &fb(3, n.as_bytes()), &fv(4, dir)]));
        let tor = cat(&[&if v1 { fb(1, &sha1::Sha1::digest(info)) } else { vec![] }, &fb(2, &s256(info)), &fb(3, blake3::hash(info).as_bytes()), &fv(4, info.len() as u64)]);
        let body = cat(&[&fb(1, b"ed3k vec"), &fb(2, b"sub"), &m(a, "a.txt", 0), &m(b"", "empty", 0), &m(b, "b.bin", 1), &fm(9, &tor)]);
        cat(&[&unhex("6564336b73657401"), &fb(1, &body)])
    };
    let (m2, mh) = (man(&v2, false), man(&hy, true));
    if let Some(d) = std::env::args().nth(2) { std::fs::write(format!("{d}/tv2.bin"), &m2[8..]).unwrap(); std::fs::write(format!("{d}/thy.bin"), &mh[8..]).unwrap(); }
    println!("\n### Version 2 and hybrid\n
The same name and piece length, with three files: `a.txt` (the sample file), `empty` (no
bytes) and `sub/b.bin` (`hello`). A file of one block has the SHA-256 of its bytes as its
`pieces root`.

```
pieces root of a.txt = {}
pieces root of b.bin = {}
file tree = d5:a.txtd0:d6:lengthi16e11:pieces root32:<root of a.txt>ee
            5:emptyd0:d6:lengthi0eee
            3:subd5:b.bind0:d6:lengthi5e11:pieces root32:<root of b.bin>eeee      ({} bytes)
root of `empty` = {}      (BLAKE3 of the empty input)
```

**Version 2:**

```
info      = d9:file tree<file tree>12:meta versioni2e4:name8:ed3k vec12:piece lengthi16384ee      ({} bytes)
btmh      = SHA-256(info) = {}
info root = BLAKE3(info)  = {}
FileSet   = {{ title = \"ed3k vec\", dirs = [\"sub\"],
              files = [ {{ root, size = 16, name = \"a.txt\" }}, {{ root, name = \"empty\" }}, {{ root, size = 5, name = \"b.bin\", dir = 1 }} ],
              torrent = {{ btmh, info = info root, info_size = {} }} }}
file      = \"ed3kset\" ‖ 01 ‖ SetFile{{ body }}      ({} bytes)
          = {}
root      = {}
```

**Hybrid.** The version 1 list has a padding file after `a.txt`, so that the next file starts
a piece. It is no member. The two pieces are `a.txt` ‖ 16,368 zero bytes, and `hello`.

```
pieces    = {}
            {}
info      = d9:file tree<file tree>
            5:filesl d6:lengthi16e4:pathl5:a.txtee
                     d4:attr1:p6:lengthi16368e4:pathl4:.pad5:16368ee
                     d6:lengthi0e4:pathl5:emptyee
                     d6:lengthi5e4:pathl3:sub5:b.binee e
            12:meta versioni2e4:name8:ed3k vec12:piece lengthi16384e6:pieces40:<pieces>e      ({} bytes; shown with spaces and line breaks that the dictionary does not have)
btih      = SHA-1(info)   = {}
btmh      = SHA-256(info) = {}
info root = BLAKE3(info)  = {}
FileSet   = as for version 2, with torrent = {{ btih, btmh, info = info root, info_size = {} }}
file      = \"ed3kset\" ‖ 01 ‖ SetFile{{ body }}      ({} bytes)
          = {}
root      = {}
```

`size` = 0 is the default value and is left out of the member `empty`.",
        hx(&s256(a)), hx(&s256(b)), tree.len(), blake3::hash(b"").to_hex(),
        v2.len(), hx(&s256(&v2)), blake3::hash(&v2).to_hex(), v2.len(), m2.len(), w(&hx(&m2), 12), blake3::hash(&m2).to_hex(),
        hx(&pieces[..20]), hx(&pieces[20..]), hy.len(), hx(&sha1::Sha1::digest(&hy)), hx(&s256(&hy)), blake3::hash(&hy).to_hex(), hy.len(), mh.len(), w(&hx(&mh), 12), blake3::hash(&mh).to_hex());
}

fn hash_body() {
    println!("\n## Hash body (http-source.md §3.1)\n
Files of the byte pattern of the tree section. `BLAKE3(body)` is the plain hash of the body
and serves as a check here; the protocol does not use it.

| Size | Blocks | Body | Length | Root of the file | BLAKE3(body) |
|---|---|---|---|---|---|");
    for (size, desc) in [(BLK + 1, "block 0 cv ‖ block 1 cv"), (2 * BLK + 1000, "node(1, 0) ‖ node(1, 1) ‖ block 0 to 2 cv; node(1, 1) is block 2 cv, carried up"), (5 * BLK, "node(2, 0) ‖ node(2, 1) ‖ node(1, 0) to node(1, 2) ‖ block 0 to 4 cv; the values of the tree section"), (4096 * BLK, "levels 11 down to 0")] {
        let mut hs = blake3::Hasher::new(); let mut cvs = vec![]; let mut off = 0;
        while off < size {
            let n = BLK.min(size - off); let d: Vec<u8> = (off..off + n).map(|i| ((i / 256) % 251) as u8).collect();
            hs.update(&d); cvs.push(blake3::Hasher::new().set_input_offset(off as u64).update(&d).finalize_non_root()); off += n;
        }
        let l = levels(&cvs); let root = root_of(&l); assert_eq!(&root, hs.finalize().as_bytes());
        let body: Vec<u8> = l.iter().rev().flat_map(|lv| lv.concat()).collect();
        println!("| {size} | {} | {desc} | {} | `{}` | `{}` |", cvs.len(), body.len(), hx(&root), blake3::hash(&body).to_hex());
    }
}
