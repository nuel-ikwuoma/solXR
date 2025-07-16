# SolanaStrategy ($SOLXR)

## What is it?

**SolanaStrategy ($SOLXR)** is a tokenized vehicle for SOL accumulation, giving $SOLXR holders a claim on a growing pool of SOL managed through fully transparent, onchain strategies.

Think of SolanaStrategy as MicroStrategy, but entirely onchain and transparent, built on Solana.

---

## TL;DR

### Deposit Pool

- The protocol starts with an initial SOL pool funded by early depositors.
- Early backers receive $SOLXR tokens, aligning their incentives with the protocol’s growth.
- **Global deposit cap** and **individual address cap** are enforced on initial deposits.

### Growth Mechanisms

- **Convertible Bonds:** Users buy bond NFTs with SOL. Proceeds are used to buy more SOL, growing the pool and increasing $SOLXR’s value.
- **ATM Offerings:** If $SOLXR trades at a premium to NAV, new tokens are sold at the market price. Proceeds are used to acquire more SOL.
- **Net Asset Value (NAV) Options:** Governance can mint options contracts that allow holders to mint $SOLXR by exchanging a proportional amount of NAV tokens.
- **Redemptions:** If $SOLXR trades at a discount to NAV, holders can vote to redeem SOL.

---

## How It Works

### 1. Convertible Bonds (SOL for SOL Acquisition)

SolanaStrategy raises funds by issuing onchain convertible bond NFTs:

- **Initial Offering:** Bonds are sold at a fixed price in SOL with a maturity date and a strike price in $SOLXR.
- **Conversion Option:** At maturity, bondholders can convert bonds into $SOLXR tokens if the token’s market price exceeds the strike price.
- **Redemption Option:** If $SOLXR’s market price does not exceed the strike price, bondholders can redeem the bonds for their principal in SOL.
- **Protocol Benefits:** SOL raised is immediately added to the pool, boosting $SOLXR’s NAV and aligning bondholder incentives with protocol growth.

### 2. At-The-Money (ATM) Offerings

If $SOLXR trades at a premium to NAV, SolanaStrategy issues new tokens to capture demand and grow the SOL pool.

- **Mechanism:** New $SOLXR tokens are sold at the market price, capped per round to maintain upside for existing holders. Proceeds are used to buy SOL.
- **Benefits:** Prevents runaway premiums and efficiently scales the SOL pool, increasing NAV for all holders.

### 3. Net Asset Value (NAV) Options

- **Mechanism:** Governance can mint options to reward contributors or market makers.
- **Benefits:** Zero cost to the platform and only valuable if $SOLXR trades at a premium to NAV.

### 4. Governance

- **Mechanism:** $SOLXR holders control the protocol from genesis. $SOLXR is both the asset and governance token.
- **Features:** No protocol fees on deposits or redemptions. Governance can pause transfers and includes a rage quit mechanism for dissenting voters.
- **Benefits:** Self-determination and transparency for all participants.

---

## Technical Overview

- **Chain:** Solana
- **Token:** $SOLXR (SPL Token)
- **Bonds:** NFT-based, convertible at maturity
- **NAV Calculation:** Based on SOL in treasury and circulating $SOLXR
- **Governance:** Anchor-based, with upgradable authority and rage quit

---

## Getting Started

1. **Initialize the Protocol:** Deploy and initialize with pool and address caps.
2. **Deposit SOL:** Early users deposit SOL and receive $SOLXR.
3. **Participate in Growth:** Buy bonds, participate in ATM rounds, or contribute to governance.
4. **Redeem or Convert:** At maturity, convert bonds to $SOLXR or redeem for SOL.

---

## Security & Transparency

- All logic is onchain and open source.
- No protocol fees; all value accrues to $SOLXR holders.
- Governance is fully decentralized and can be exited via rage quit.
