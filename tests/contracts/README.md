# Ledger tests

Foundry tests of [`contracts/CreditLedger.sol`](../../contracts/CreditLedger.sol) and the stake
token it creates, [`contracts/StakeToken.sol`](../../contracts/StakeToken.sol)
([`extensions/credit.md`](../../extensions/credit.md) §6): unit tests, fuzz tests, invariants,
and the regression tests of the 2026-10-18 audit. The contracts are tested in place, not copied.
The stake token's tests are in `test/Stake.t.sol`.

## Setup

- Foundry (`forge`, `cast`, `anvil`), tested with 1.5.1 and 1.8.6. `foundry.toml` turns
  `dynamic_test_linking` off: 1.8 rewrites `new` in tests and then sees two `IOperators`
  types. 1.8 also runs each call of a test as its own transaction, so storage is cold; the
  re-entering mock minter keeps its calldata in code to fit the 60,000 gas of a mint.
- solc 0.8.37. Foundry fetches it, or point `FOUNDRY_SOLC` at a local binary and add `--offline`.
- forge-std in `lib/` (not tracked):

```
git clone https://github.com/foundry-rs/forge-std lib/forge-std
git -C lib/forge-std checkout b013eff2ac4fce26f34d789e6f5c49021c5ee817
```

## Run

```
forge test                  # optimizer on, 200 runs, as the gas figures of credit.md §11
forge test --profile ir     # via-IR
python3 -I gas/measure.py   # gas of each call, one transaction each, on its own anvil
```

`gas/measure.py` takes `FOUNDRY_BIN` (directory of the Foundry binaries; default: `PATH`) and
`ANVIL_PORT` (default 8547). It puts a no-op at the native minter address, so the figures leave
out the minter's own cost (about 30,000 per mint on Subnet-EVM).

Under `--profile ir` a few tests that `vm.warp` fail because the IR optimizer reuses
`block.timestamp` inside the test function; that is the test code, not the contract.
