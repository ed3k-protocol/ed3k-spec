// Credit signatures (credit.md §3, §4.2, §6.5): EIP-712 domain, Bid proof, Receipt and Hold signatures with
// secp256k1 key 1, keccak256, RFC 6979 low-s ECDSA. Prints the section of ../vectors-v1.md as markdown.
use super::*;
use ext::w;
use k256::ecdsa::{SigningKey as K1, VerifyingKey};
use sha3::Keccak256;

fn keccak(d: &[u8]) -> [u8; 32] { Keccak256::digest(d).into() }
fn word(b: &[u8]) -> Vec<u8> { let mut o = vec![0u8; 32 - b.len()]; o.extend(b); o }          // left padded: address, uint
fn word_r(b: &[u8]) -> Vec<u8> { let mut o = b.to_vec(); o.resize(32, 0); o }                  // right padded: bytesN
fn uint(v: u64) -> Vec<u8> { word(&v.to_be_bytes()) }
fn address(vk: &VerifyingKey) -> Vec<u8> { keccak(&vk.to_encoded_point(false).as_bytes()[1..])[12..].to_vec() }

// r ‖ s ‖ v, s in the lower half, v 27 or 28; checked by recovery
fn sign(sk: &K1, digest: &[u8; 32]) -> Vec<u8> {
    let (sig, rec) = sk.sign_prehash_recoverable(digest).unwrap();
    assert!(sig.normalize_s().is_none(), "s not low");
    let back = VerifyingKey::recover_from_prehash(digest, &sig, rec).unwrap();
    assert_eq!(&back, sk.verifying_key());
    cat(&[&sig.to_bytes(), &[27 + rec.to_byte()]])
}

pub fn run() {
    let sk = K1::from_slice(&word(&[1])).unwrap();
    let payer = address(sk.verifying_key());
    assert_eq!(hx(&payer), "7e5f4552091a69125d5dfcb7b8c2659029395bdf");
    let payee = [0xb2u8; 20];
    let contract = unhex("5fbdb2315678afecb367f032d93f642f64180aa3");
    // CREATE address of nonce 0 of the first default account of a local test chain: keccak(rlp([sender, 0]))
    let deployer = unhex("f39fd6e51aad88f6f4ce6ab8827279cfffb92266");
    assert_eq!(keccak(&cat(&[&[0xd6, 0x94], &deployer, &[0x80]]))[12..], contract[..]);

    let dom_type = keccak(b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
    let sep = keccak(&cat(&[&dom_type, &keccak(b"ed3k credit"), &keccak(b"1"), &uint(31337), &word(&contract)]));
    let digest = |sh: &[u8; 32]| keccak(&cat(&[b"\x19\x01", &sep, sh]));

    let bid_type = keccak(b"Bid(address payer,address payee,bytes16 challenge)");
    let challenge = unhex("00112233445566778899aabbccddeeff");
    let bid_sh = keccak(&cat(&[&bid_type, &word(&payer), &word(&payee), &word_r(&challenge)]));
    let bid_d = digest(&bid_sh); let proof = sign(&sk, &bid_d);

    let rcpt_type = keccak(b"Receipt(address issuer,address holder,uint32 day,uint64 kib,uint64 spent)");
    let (day, kib, spent) = (20739u64, 4096u64, 0u64);
    let rcpt_sh = keccak(&cat(&[&rcpt_type, &word(&payer), &word(&payee), &uint(day), &uint(kib), &uint(spent)]));
    let rcpt_d = digest(&rcpt_sh); let sig = sign(&sk, &rcpt_d);
    // the first receipt of a queue entry (§3.1), with proof; spent = 0 is left out
    let msg = cat(&[&fv(1, day), &fv(2, kib), &fv(3, spent), &fb(4, &sig), &fb(5, &proof)]);

    let hold_type = keccak(b"Hold(address payer,address payee,uint256 amount,uint256 nonce,uint256 deadline)");
    let (amount, nonce, deadline) = (16_000_000u64, 1_791_849_600_123u64, 1_791_850_200u64);
    let hold_sh = keccak(&cat(&[&hold_type, &word(&payer), &word(&payee), &uint(amount), &uint(nonce), &uint(deadline)]));
    let hold_d = digest(&hold_sh); let hold_sig = sign(&sk, &hold_d);
    // SignedHold of a CreditRelay (§4.3)
    let hold_msg = cat(&[&fb(1, &payer), &fb(2, &payee), &fv(3, amount), &fv(4, nonce), &fv(5, deadline), &fb(6, &hold_sig)]);

    // values two independent implementations agree on
    assert_eq!(hx(&sep), "c5a889472a3440d210993204c686a1fce378293f73da5de5a63d45654d2d3d17");
    assert_eq!(hx(&bid_d), "0d5e17820d5a72e8614687603b98523285e39f17603bc7d4760cdabbf7d97eda");
    assert_eq!(hx(&proof), "d21c085f283e12a11432bd5f50338417d6ac6296d4404f7be911978261d18f4d39deda80fad066dc3b15703848f4c0fd9c0e3cd4ad6317aa2397917ed6425b7f1c");
    assert_eq!(hx(&hold_d), "c7049a77c9f28df00326b79925e53ea36a2cb88ba9b0981c1a47597a96f39f69");
    assert_eq!(hx(&hold_sig), "757fced1a49373539fe6c050b2f449c93bba8889d4fba658857885a9e9cc8eeb12363644cc1e9df8e603b0bead450ee85c838b924430ce514ccefbe0822a5e731b");
    assert_eq!(hx(&sig), "ea5a302dda933e0388e87b65d465e8a292349ecb7b2bdcf2733e3a131a9770d913ebf1ba788a5c2f502a3d7418552582b38b30427d08140220d195d9df6762031b");

    println!("
## Credit signatures (credit.md §3, §4.2, §4.3 and §6.5)

The ledger domain is a test domain, not the public network: chain ID 31337 and the address of
the first contract that the first default account of a local test chain creates (nonce 0).
The credit key is the secp256k1 private key 1; it is the issuer and `payer`. The uploader's
address is 20 bytes `b2`. Signatures are ECDSA over the digest with the nonce of RFC 6979
(HMAC-SHA-256), `r ‖ s ‖ v` with `s` in the lower half, and recover to the issuer's address.
The words of the EIP-712 encoding are 32 bytes: an address and an integer padded on the left,
`bytes16` on the right.

```
domain       = {{ name = \"ed3k credit\", version = \"1\", chainId = 31337,
                 verifyingContract = {} }}
domain type  = keccak256(\"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)\")
             = {}
separator    = keccak256(domain type ‖ keccak256(name) ‖ keccak256(version) ‖ chainId ‖ verifyingContract)
             = {}
issuer       = keccak256(x ‖ y of the public key)[12..32) = {}
digest       = keccak256(1901 ‖ separator ‖ struct hash)
```

**Bid** (§4.2), the `proof` of the first receipt, with the `challenge` `{}`:

```
type hash    = keccak256(\"Bid(address payer,address payee,bytes16 challenge)\")
             = {}
struct hash  = keccak256(type hash ‖ payer ‖ payee ‖ challenge)
             = {}
digest       = {}
proof        = {}
               {}
```

**Receipt** (§3) for day 20739, `kib` = 4096, `spent` = 0, sent with `proof` as the first
receipt of a queue entry (§3.1). `spent` = 0 is left out of the message:

```
type hash    = keccak256(\"Receipt(address issuer,address holder,uint32 day,uint64 kib,uint64 spent)\")
             = {}
struct hash  = keccak256(type hash ‖ issuer ‖ holder ‖ day ‖ kib ‖ spent)
             = {}
digest       = {}
sig          = {}
               {}
Receipt      = {{ day = 20739, kib = 4096, sig, proof }}
             = {}
               ({} bytes)
```",
        hx(&contract), hx(&dom_type), hx(&sep), hx(&payer),
        hx(&challenge), hx(&bid_type), hx(&bid_sh), hx(&bid_d), hx(&proof[..32]), hx(&proof[32..]),
        hx(&rcpt_type), hx(&rcpt_sh), hx(&rcpt_d), hx(&sig[..32]), hx(&sig[32..]), w(&hx(&msg), 15), msg.len());

    println!("
**Hold** (§6.5), the signature of `holdFor` and the `SignedHold` of a `CreditRelay` (§4.3), for
16 credits (`amount` = {amount} µ), `nonce` = {nonce} (a time in ms) and
`deadline` = {deadline}, 10 min later:

```
type hash    = keccak256(\"Hold(address payer,address payee,uint256 amount,uint256 nonce,uint256 deadline)\")
             = {}
struct hash  = keccak256(type hash ‖ payer ‖ payee ‖ amount ‖ nonce ‖ deadline)
             = {}
digest       = {}
sig          = {}
               {}
SignedHold   = {{ payer, payee, amount, nonce, deadline, sig }}
             = {}
               ({} bytes)
```",
        hx(&hold_type), hx(&hold_sh), hx(&hold_d), hx(&hold_sig[..32]), hx(&hold_sig[32..]), w(&hx(&hold_msg), 15), hold_msg.len());
}
