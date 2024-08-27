#!/bin/bash

# ➜  injective-auction-pool git:(main) ✗ injectived config
# {
#         "chain-id": "injective-777",
#         "keyring-backend": "os",
#         "output": "json",
#         "node": "https://devnet.tm.injective.dev:443",
#         "broadcast-mode": "sync"
# }

export FROM=devnet
export FROM_ADDR=inj14au322k9munkmx5wrchz9q30juf5wjgz2cfqku

# 1) optimize both contracts
#
just optimize


# 2) store the treasure chest contract
#
TREASURE_TX_HASH=$(injectived tx wasm store ./artifacts/treasurechest_contract.wasm --from ${FROM} --yes --gas-prices "500000000inj" --gas-adjustment 1.3 --gas auto | jq -r '.txhash')
echo $TREASURE_TX_HASH
# TREASURE_TX_HASH=634BB9BA7186D46BD74A08F0FAFBB9EB6811CF657477A2004EB0BDCA666FADE9

export TREASURE_CODE_ID=$(injectived query tx ${TREASURE_TX_HASH} |jq -e -r ' .events[] | select(.type=="cosmwasm.wasm.v1.EventCodeStored").attributes[] | select(.key=="code_id").value ' | tr -d '"')
echo $TREASURE_CODE_ID
# TREASURE_CODE_ID=18


# 3) store the auction pool contract
#
AUCTION_TX_HASH=$(injectived tx wasm store ./artifacts/injective_auction_pool.wasm --from ${FROM} --yes --gas-prices "500000000inj" --gas-adjustment 1.3 --gas auto | jq -r '.txhash')
echo $AUCTION_TX_HASH
# TX_HASH_AUCTION=FDE2C32FE3058F60AEEB2B2DB07FE7D83EA229094DC0DFF9A648526EE419F328

export AUCTION_CODE_ID=$(injectived query tx ${AUCTION_TX_HASH} |jq -e -r ' .events[] | select(.type=="cosmwasm.wasm.v1.EventCodeStored").attributes[] | select(.key=="code_id").value ' | tr -d '"')
echo $AUCTION_CODE_ID
# AUCTION_CODE_ID=19

# 4) instantiate the auction pool contract
#
export FEE_AMT=$(injectived query tokenfactory params|jq -r '.params.denom_creation_fee[0].amount')
echo $FEE_AMT
#export FEE_AMT=10000000000000000000

INIT_MSG=$(envsubst < ./deploy/testnet/auction.json)
date_label=$(date +"%Y-%m-%d %H:%M")
echo $INIT_MSG

AUCTION_INIT_TX_HASH=$(injectived tx wasm instantiate $AUCTION_CODE_ID ${INIT_MSG} \
    --label auction_${date_label} \
    --admin $FROM_ADDR \
    --amount ${FEE_AMT}inj \
    --from $FROM --yes --gas-prices "500000000inj" --gas-adjustment 1.3 --gas auto | jq -r '.txhash')
echo $AUCTION_INIT_TX_HASH
# AUCTION_INIT_TX_HASH=B9BD43827063E56CC61F84EE43A23450922B9ABE98AAC1C2EE6EEB36ED612A0D

export AUCTION_CONTRACT=$(injectived query tx $AUCTION_INIT_TX_HASH | jq -r '.events[]| select(.type=="cosmwasm.wasm.v1.EventContractInstantiated").attributes[] |select(.key=="contract_address").value '|tr -d '"')
echo $AUCTION_CONTRACT
# AUCTION_CONTRACT=inj1puua6kj5v5vtrglpumr4evlug4sl8tpsk46sar

injectived query wasm cs smart $AUCTION_CONTRACT '{"current_auction_basket":{}}' | jq
injectived query wasm cs smart $AUCTION_CONTRACT '{"whitelisted_addresses":{}}' | jq
injectived query wasm cs smart $AUCTION_CONTRACT '{"funds_locked":{}}' | jq
injectived query wasm cs smart $AUCTION_CONTRACT '{"bidding_balance":{}}' | jq
injectived query wasm cs smart $AUCTION_CONTRACT '{"config":{}}' | jq
injectived query wasm cs smart $AUCTION_CONTRACT '{"unsettled_auction":{}}' | jq
injectived query wasm cs smart $AUCTION_CONTRACT '{"treasure_chest_contracts":{}}' | jq

injectived query wasm cs smart $tc '{"config":{}}' | jq

# join pool with 100inj
injectived tx wasm execute $AUCTION_CONTRACT '{"join_pool":{"auction_round":1}}' \
  --from $FROM --yes --gas-prices "500000000inj" --gas-adjustment 1.3 --gas auto \
  --amount 100000000000000000000inj | jq

# exit pool
injectived tx wasm execute $AUCTION_CONTRACT '{"exit_pool":{}}' --from $FROM --yes \
  --gas-prices "500000000inj" --gas-adjustment 1.3 --gas auto --amount 1000000000000000000factory/$AUCTION_CONTRACT/auction.0 | jq

# join pool again with 1inj
injectived tx wasm execute $AUCTION_CONTRACT '{"join_pool":{"auction_round":0}}' \
  --from $FROM --yes --gas-prices "500000000inj" --gas-adjustment 1.3 --gas auto \
  --amount 1000000000000000000inj | jq

# try bid with basket value of 10_000
injectived tx wasm execute $AUCTION_CONTRACT '{"try_bid":{"auction_round":1, "basket_value":"1000000000000000000000"}}' \
  --from $FROM --yes --gas-prices "500000000inj" --gas-adjustment 1.3 --gas auto | jq

# settle auction
injectived tx wasm execute $AUCTION_CONTRACT '{"settle_auction":{"auction_round":1, "auction_winner":"inj1puua6kj5v5vtrglpumr4evlug4sl8tpsk46sar", "auction_winning_bid": "100250000000000000001"}}' --from $FROM --yes --gas-prices "500000000inj" --gas-adjustment 1.3 --gas auto --amount $FEE_AMT"inj" | jq 

# withdraw from treasure chest
injectived tx wasm execute $TREASURE_CHEST_CONTRACT '{"withdraw":{}}' --from $FROM --yes --gas-prices "500000000inj" --gas-adjustment 1.3 --gas auto --amount 1000000000000000000factory/$AUCTION_CONTRACT/auction.0 | jq
