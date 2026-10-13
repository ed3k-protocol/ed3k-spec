# The credit ledger: why a chain, which one, and who runs a node

**Status:** design note (2026-10-12). Informative. Nothing here is deployed or measured.
**Relates to:** [`extensions/credit.md`](../extensions/credit.md). Section numbers (§) refer to this document unless a document is named.

The credit extension keeps balances in a contract on an EVM chain that runs as an Avalanche L1
(formerly "subnet"). This note gives the reasons and the alternatives, and answers one question
first.

## 1. Does anyone need a blockchain node?

| Who | Needs a node? | What it runs |
|---|---|---|
| A user of ed3k without the extension | no | nothing |
| A user with the extension | **no** | the ed3k node, with the URLs of one or more RPC endpoints |
| A user who trusts nobody's answers | optional | a node that follows the L1, on the same machine or elsewhere |
| An operator | **yes** | a validator of the L1, and an RPC endpoint for others |

- **The ledger node is a separate program.** The ed3k node speaks Ethereum JSON-RPC to it and
  does not care whether it runs on the same machine. A chain grows without end, and its
  software has its own release cycle; neither belongs in a file sharing client.
- **A user does not need one.** Everything that moves credits is a signature made by the ed3k
  node. A remote node can only lie about what it reads, and that is bounded
  ([`credit.md`](../extensions/credit.md) §7.3).
- **The network needs several.** An L1 of one's own has no validators but its own. Someone
  has to run them, around the clock.
- **A node that only follows the L1** syncs the P-Chain of Avalanche and the L1, not the rest
  of the primary network (`--partial-sync-primary-network`, `--track-subnets`). Its disk use
  was not measured.

So the honest form of the question is the next one.

## 2. Is a chain needed at all?

What the ledger must do: hold one balance per key that every uploader reads the same way,
apply decay and limits that no client can skip, and refuse a credit that was spent twice.

| | Pro | Con |
|---|---|---|
| **No ledger.** Each uploader checks receipts itself; double spends are bounded by limits and by fraud records in Kad | no nodes, no operators, no public trade graph | rules are each uploader's policy, so a modified client skips decay; a receipt can be shown to many uploaders; transfers are not possible |
| **One server** that signs balances | simple | whoever runs it can mint, and it is one point to take down |
| **A set of servers that vote**, with a protocol of our own | no dependency | this is a consensus protocol, the hardest part of a chain, written again |
| **A contract on an existing chain** | its validators are not ours to recruit; nothing to run | gas is real money, paid by someone; rules of that chain change without us |
| **A chain of our own** (chosen) | gas costs nothing that has a price; the contract is the rule book, readable by everyone; standard tools and RPC | needs operators; is exactly as honest as they are (§3) |

**A chain of one's own is a replicated database with a public rule book.** It does not get its
security from the rest of Avalanche: the P-Chain only records who the validators are. What it
gives over a set of servers is consensus that exists and is tested, the EVM as a precise
statement of the rules, and a history anyone can re-check.

## 3. Who validates

| | Pro | Con |
|---|---|---|
| **Operators admitted by vote** (chosen) | no token with a price; no key of the project after genesis; stock software | permissioned; someone has to find the first operators |
| Proof of stake in a token of the chain | anyone may validate | the token needs a market value to secure anything, which makes the project an issuer; a cheap token is a cheap takeover |
| Stake in credits | no second token | credits decay and are farmable within limits; the ledger would secure itself with what it defines |

**What the operators can do**

- Two thirds of them can change the validator set and, by running other software, any rule.
  They can censor: leave transactions out.
- **They cannot forge a receipt or a transfer.** Those need the signature of a credit key, and
  every node that follows the chain checks it. A chain with forged entries is rejected by
  honest nodes and by anyone who re-checks it.
- Each operator's own credit address is mature from the start
  ([`credit.md`](../extensions/credit.md) §6.3). That is one address each, with the weekly
  limit of any issuer.

**What it costs them.** Each validator of an L1 pays a running fee to the P-Chain: 512 nAVAX
per second at the lowest price, about 1.33 AVAX in 30 days, and more when many L1 validators
exist (`genesis/genesis_mainnet.go`, `ValidatorFeeConfig`, in the avalanchego tree). To this
comes a machine that is always on. Five validators let the chain go on with one down; that
number is a rule of thumb, not something measured here.

## 4. Why Avalanche, and why the EVM

| | Pro | Con |
|---|---|---|
| **Avalanche L1 with Subnet-EVM** (chosen) | an own chain with own gas rules is a supported product; fast finality, so a read is final; the EVM brings `ecrecover`, EIP-712, ERC-20 and every wallet and explorer | validators pay AVAX every month; the software follows Avalanche's upgrades |
| A rollup on Ethereum with its own sequencer | state is anchored in Ethereum | one sequencer is one operator; fees for posting data are real money |
| A contract on a public L2 or on the C-Chain | nothing to run | gas in a coin with a price, for every settlement of every user |
| A chain from another kit, without the EVM | a native Ed25519 check; smaller state | no standard signature format, no tools; all of it to be written |

From the avalanchego tree (commit `1c2042de4`, 2026-10-09):

- **Gas price.** `graft/subnet-evm/commontype/gas_price_config.go` has a static price and
  refuses a minimum of 0. The lowest is 1 wei, so an account needs a little native coin.
- **Native minter.** `precompile/contracts/nativeminter` lets an allowed contract mint the
  native coin. The gas reward uses it ([`credit.md`](../extensions/credit.md) §6.6).
- **Who may deploy.** `precompile/contracts/deployerallowlist` limits deployment to allowed
  accounts. It keeps other contracts off the chain.
- **Who may send.** `precompile/contracts/txallowlist` could limit senders. It is not used
  (§6).
- **Signatures.** There is a P-256 check at address `0x100`
  (`params/hooks_libevm.go`) and none for Ed25519.

**Not verified:** that `latest` in the RPC means an accepted block by default, and the
interface of the validator manager contract, which is not in that tree.

## 5. The key

ed3k node keys are Ed25519 (`Protocol.md` §6.1). The EVM checks secp256k1.

| | Pro | Con |
|---|---|---|
| **A second key, secp256k1** (chosen) | stock chain; about 3,000 gas per signature; EIP-712 as every tool knows it; the ledger never sees a node ID | a second key to keep safe |
| Ed25519 by an added function of the chain | one key | every validator and RPC node runs a build of our own, kept up with each Avalanche upgrade; node IDs, which Kad maps to host addresses, would be on the ledger |
| Ed25519 checked by contract code | one key, stock chain | several hundred thousand gas per signature; few receipts per block |
| P-256, which the chain can check | stock chain | still a second key, and no EIP-712 tools |

That the ledger does not see node IDs is worth more than the saved key. It does not hide much
from a peer ([`credit.md`](../extensions/credit.md) §9), but it keeps the public record free of
anything Kad can resolve.

## 6. Gas

Gas has no price worth naming, but it cannot be nothing: a chain where sending is free is
filled by whoever wants to.

| | Pro | Con |
|---|---|---|
| **Anyone sends; the ledger mints coin for useful work** (chosen) | plain JSON-RPC, no gate; sending junk costs coin that only real settlements earn | untried; the reward must stay below what it would pay to fill blocks with small receipts |
| Only operators send; clients hand them signed batches | simplest control | a second interface beside the RPC; operators can delay anyone |
| A faucet gives coin to whoever asks | trivial | nothing stops a flood |

- A node with no coin asks a peer to send for it, over ed3k
  ([`credit.md`](../extensions/credit.md) §4.3). The peer is paid by the reward.
- The reward needs a receipt that mints or burns at least 1 credit. Minting needs a mature
  issuer and is capped per week, so the coin one key can earn is capped too.

## 7. Transfers

| | Pro | Con |
|---|---|---|
| Bound to the key | farming earns priority only | a lost key loses the credits |
| One move to a new key | survives a reinstall | also a way to sell an account |
| **Transferable, ERC-20 with decay** (chosen) | gifts, pooling, a new key at any time | credits can get a price; farming is then paid; law on tokens may apply |

Rules that follow from transfers ([`credit.md`](../extensions/credit.md) §6.5):

| Without it | Rule |
|---|---|
| passing credits to and fro keeps a hoard "active" | receiving is never activity |
| 1,000 credits split over 100 keys sit under the floor of 10 and never decay | transferred credits are a second amount per account that decays to zero |
| maturity is bought | only minted credits count for maturity |
| credits move at no cost | 10 % of a transfer is burned, about one day of decay |
| a pool at an exchange holds credits | every address decays, contracts too |
| credits shown for a bid are moved away before the uploader settles | a hold (§6.5 there) locks one turn per uploader for an hour; a transfer cannot dip into it |

The standard interface is kept so that wallets and explorers show a balance. Two things differ
from a plain ERC-20 and have to be said to anyone who integrates it: the balance falls without
an event, and a transfer delivers less than was sent.

## 8. Money was left out

A design in which a downloader pays the uploader in a stable coin is proof against fake
identities: paying oneself gains nothing. It was not taken.

| | Credits (chosen) | Payments |
|---|---|---|
| Fake identities | bounded by maturity and limits | no gain at all |
| Setup for a user | none | a funded wallet |
| What is public | who traded how much with whom | the same, and the wallet leads to a name at an exchange |
| The uploader | earns priority | is paid for distributing files, which is a far worse position in law |
| The chain | own, gas without a price | one where the coin exists, gas with a price |

## 9. What happens to spent credits

A bidder's payment is taken from its balance at settlement. Either all of it is burned, or a
tenth goes to the uploader and the rest is burned.

| | Burn all | 10 % to the uploader |
|---|---|---|
| Why an uploader sells priority | no reason of its own; it does what the client does | it earns a little |
| Why an uploader settles a payment | only together with the mint of the same receipt. Where that mint is nothing (issuer immature or at its weekly limit), nothing but the gas reward | the share, in every case |
| False bids | the uploader gains nothing by inventing a bid or by charging the full maximum, so an unmodified second-price rule is the natural behaviour | the uploader gains a tenth of every µ it charges more. A modified uploader charges the maximum, and bidders learn to bid lower than they would pay |
| Slots | no reason to exceed the share for bidders | a reason to give bidders more than two thirds |
| Fake identities | no effect | none either: paying one's own second key loses 90 %, worse than a transfer |
| Supply of credits | falls with every purchase and by decay | falls nine tenths as fast |
| What a credit is | priority that is used up | still that, with a small payment for capacity in it |
| The uploader in law | earns nothing for a slot | is paid, in a transferable unit, for serving a file sooner |

- The share arrives as transferred credit: it decays to zero, is not activity and does not
  count for maturity. So it cannot keep a hoard alive or buy maturity.
- With the whole payment for the uploader, each row on the right gets ten times worse, and
  paying oneself costs nothing. That was not considered further.

**Decided: 10 %.** One signed object already ties the burn to the mint, but not for
receipts of issuers that are immature or at their limit, and those are exactly the buyers
whose payments would otherwise stay unsettled. The gain from a false bid is real but small,
and it is bounded by the bidder's own maximum. The row on law is the one that could change
this. The share is a constant of the contract.

## 10. Open points

- Who the first operators are, and how the operator contract and the validator manager are
  wired. Neither is specified.
- Whether credits that can be sold are acceptable in law. Not checked.
- A light check of a remote node by the validators' signatures.
- Measurements: gas per receipt, state per year, disk of a node that follows the L1.
