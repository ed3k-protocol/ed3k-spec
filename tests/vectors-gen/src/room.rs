// Rooms (room.md §1.2, §2, §3 and §4). Prints the section of ../vectors-v1.md as markdown.
use super::*;
use ext::{conv, w};

fn sig2(s: &[u8]) -> String { format!("{}\n              {}", hx(&s[..32]), hx(&s[32..])) }

pub fn run(pk1: &[u8], pk2: &[u8], seed2: &[u8]) {
    let sign = |seed: &[u8], m: &[u8]| { let sk = SigningKey::from_bytes(seed.try_into().unwrap()); let s = sk.sign(m); sk.verifying_key().verify_strict(m, &s).unwrap(); s.to_bytes().to_vec() };
    let secret: Vec<u8> = (0..32u8).map(|i| 0x37 ^ i).collect();
    let id = b"sid1";
    let peer = h("ed3k v1 node id", pk1)[..16].to_vec();
    let joiner = h("ed3k v1 node id", pk2)[..16].to_vec();
    assert_eq!(hx(&joiner), "017da93e9fc9e3079a462d8b461dfc9b");
    let t: u64 = 1791849600; let hour = (t / 3600).to_be_bytes(); let next = (t / 3600 + 1).to_be_bytes();
    let minute = (t / 60).to_be_bytes();

    // names (§1.2)
    let public = h("ed3k v1 room", &cat(&[pk1, id]))[..16].to_vec();
    let k = h("ed3k v1 room private", &cat(&[&secret, pk1, id]));
    let name = h("ed3k v1 room private name", &k)[..16].to_vec();
    let target = h("ed3k v1 room private target", &cat(&[&k, &hour]))[..16].to_vec();
    let target2 = h("ed3k v1 room private target", &cat(&[&k, &next]))[..16].to_vec();
    let k_data = h("ed3k v1 room private data", &k);

    // secret proofs (§4.1)
    let pj = h("ed3k v1 room join", &cat(&[&k, &joiner, &peer]))[..16].to_vec();
    let pa = h("ed3k v1 room answer", &cat(&[&k, &joiner, &peer]))[..16].to_vec();

    // member proof (§4.2): the joiner's member key is the second key
    let member_id = h("ed3k v1 node id", pk2)[..16].to_vec();
    let mpre = cat(&[b"ed3k v1 room proof", &name, &joiner, &peer, &minute]);
    let msig = sign(seed2, &mpre);
    let proof_msg = cat(&[&fv(1, t / 60), &fb(2, &msig)]);

    // generation and wrap (§3.2): moderator's ephemeral pair is (x, e)
    let g: Vec<u8> = (0..32u8).map(|i| 0x6b ^ i).collect();
    let key_id = h("ed3k v1 room key id", &cat(&[&k, &g]))[..8].to_vec();
    let k_post = h("ed3k v1 room generation", &cat(&[&k, &g]));
    let x = unhex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    let e = x25519_base(&x);
    let (mx_pub, mx_priv) = conv(seed2);
    let ss = x25519(&x, &mx_pub); assert_eq!(ss, x25519(&mx_priv, &e)); assert_ne!(ss, [0u8; 32]);
    let gen: u64 = 1;
    let kw = h("ed3k v1 room wrap", &cat(&[&ss, &e, pk2, &gen.to_be_bytes()]));
    let wrapped = aead(&kw, &[0u8; 12], &g); assert_eq!(wrapped.len(), 48);
    let wraps = cat(&[&member_id, &wrapped]);
    let drop = cat(&[&fv(1, gen), &fb(2, &key_id), &fv(3, t), &fb(4, &e), &fb(5, &wraps)]);

    // sealed posts (§3): closed room under k_post, private room under k_data
    let nonce = unhex("000102030405060708090a0b");
    let text = cat(&[&fb(1, b"hello room")]);
    let sealed = cat(&[&nonce, &chacha(&k_post, &nonce, &text)]);
    let sealed_p = cat(&[&nonce, &chacha(&k_data, &nonce, &text)]);
    let post = cat(&[&fv(1, t), &fb(3, &sealed), &fv(4, gen), &fb(5, &key_id)]);
    let body = fm(5, &post);
    let ipre = cat(&[b"ed3k v1 room item", &name, &body]);
    let isig = sign(seed2, &ipre);
    let item = cat(&[&fb(1, &body), &fb(2, pk2), &fm(3, &sigmsg(&isig))]);
    let post_id = h("ed3k v1 room post", &cat(&[&name, &body]))[..16].to_vec();

    println!("
## Rooms (room.md §1.2, §2, §3 and §4)

The room has the sample key as its room key, the `id` `sid1` and the sample secret. The
joiner is the node of the second key, `{}`; the peer is the node of the sample key,
`{}`. The joiner's member key is the second key as well, so its member ID is its node ID.
room.md §1.1 forbids a node key as a member key; the reuse is for the vectors only. The sample
time lies in hour 497736 (`{}`) and minute 29864160.

**Names** (room.md §1.2):

```
room (public) = H(\"ed3k v1 room\", key ‖ \"sid1\")[0..16) = {}
k             = H(\"ed3k v1 room private\", secret ‖ key ‖ \"sid1\") = {}
room          = H(\"ed3k v1 room private name\", k)[0..16) = {}
target        = H(\"ed3k v1 room private target\", k ‖ {})[0..16) = {}
next hour     = H(\"ed3k v1 room private target\", k ‖ {})[0..16) = {}
k_data        = H(\"ed3k v1 room private data\", k) = {}
```

`room` of the private room is the one every item of the private and the closed room is signed
under below.

**Secret proofs** (room.md §4.1). The joiner's node ID comes first:

```
RoomJoin.proof  = H(\"ed3k v1 room join\",   k ‖ joiner ‖ peer)[0..16) = {}
RoomState.proof = H(\"ed3k v1 room answer\", k ‖ joiner ‖ peer)[0..16) = {}
```

**Member proof** (room.md §4.2), sent by the joiner:

```
signed      = \"ed3k v1 room proof\" ‖ room ‖ joiner ‖ peer ‖ {}      ({} bytes)
signature   = {}
RoomProof   = {}
              ({} bytes)
```

**Generation 1** (room.md §3.2). `g` is 32 bytes, byte *i* = `0x6b XOR i`. The moderator's
X25519 pair for this `KeyDrop` is `(x, e)` of the sample inputs; the member's X25519 key is the
second key converted (Protocol.md §6.4):

```
g           = {}
key_id      = H(\"ed3k v1 room key id\", k ‖ g)[0..8) = {}
k_post      = H(\"ed3k v1 room generation\", k ‖ g) = {}
x member    = {}
X25519      = X25519(x, x member) = {}
kw          = H(\"ed3k v1 room wrap\", X25519 ‖ e ‖ member key ‖ 0000000000000001) = {}
wrap        = ChaCha20-Poly1305(kw, 000000000000000000000000, g) = {}
KeyDrop     = {{ gen = 1, key_id, issued = time, e, wraps = member ID ‖ wrap }}
            = {}
              ({} bytes)
```

**A sealed post** (room.md §2.5, §3). The text is the 10 ASCII bytes `hello room`, sealed with
the sample nonce. A member signs the item with its member key:

```
RoomText    = {}
sealed      = {}      closed room, under k_post
sealed      = {}      private room, under k_data
RoomPost    = {{ sent = time, sealed, gen = 1, key_id }}
            = {}
RoomBody    = {{ post }} = {} bytes
signed      = \"ed3k v1 room item\" ‖ room ‖ RoomBody      ({} bytes)
signature   = {}
RoomItem    = {}
              ({} bytes)
post id     = H(\"ed3k v1 room post\", room ‖ RoomBody)[0..16) = {}
```",
        hx(&joiner), hx(&peer), hx(&hour),
        hx(&public), hx(&k), hx(&name), hx(&hour), hx(&target), hx(&next), hx(&target2), hx(&k_data),
        hx(&pj), hx(&pa),
        hx(&minute), mpre.len(), sig2(&msig), w(&hx(&proof_msg), 14), proof_msg.len(),
        hx(&g), hx(&key_id), hx(&k_post), hx(&mx_pub), hx(&ss), hx(&kw), hx(&wrapped), w(&hx(&drop), 14), drop.len(),
        hx(&text), hx(&sealed), hx(&sealed_p), w(&hx(&post), 14), body.len(), ipre.len(), sig2(&isig), w(&hx(&item), 14), item.len(), hx(&post_id));
}
