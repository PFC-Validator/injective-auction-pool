#!/bin/bash

# ╰─ ./deploy/migrate.sh -c $AUCTION_CONTRACT -f ww

# Function to display usage
usage() {
    echo "Usage: $0 -c <contract_address> -f <from_key_name>"
    echo "  -c <contract_address>      : Address of the contract to migrate"
    echo "  -f <from_key_name>         : Key name of the sender"
    exit 1
}

# Parse command line arguments
while getopts "c:f:" opt; do
    case $opt in
        c) CONTRACT_ADDRESS="$OPTARG" ;;
        f) FROM_KEY_NAME="$OPTARG" ;;
        *) usage ;;
    esac
done

# Check if mandatory arguments are provided
if [ -z "$CONTRACT_ADDRESS" ] || [ -z "$FROM_KEY_NAME" ]; then
    usage
fi

# Step 1: Compile the contract
echo "Compiling and optimize the contract..."
just  optimize


# Path to the compiled contract
WASM_FILE="artifacts/injective_auction_pool.wasm"
if [ ! -f "$WASM_FILE" ]; then
    echo "Compiled WASM file not found at $WASM_FILE!"
    exit 1
fi

# Step 2: Store the optimized contract on the blockchain
echo "Storing the contract..."
STORE_RESULT=$(injectived tx wasm store "$WASM_FILE" --from $FROM_KEY_NAME --yes --gas-prices "160000000inj" --gas-adjustment 1.3 --gas auto -o json)
STORE_TXHASH=$(echo "$STORE_RESULT" | jq -r '.txhash')
echo "Store transaction hash: $STORE_TXHASH"

# Step 3: Get the code ID from the transaction result
echo "Fetching code ID..."
sleep 5 # wait for the transaction to be processed
CODE_ID=$(injectived query tx $STORE_TXHASH -o json | jq -r '.events[] | select(.type=="cosmwasm.wasm.v1.EventCodeStored").attributes[] | select(.key=="code_id").value' | tr -d '"')
if [ -z "$CODE_ID" ]; then
    echo "Failed to retrieve code ID!"
    exit 1
fi
echo "Code ID: $CODE_ID"

# Step 4: Migrate the contract
echo "Migrating the contract..."
MIGRATE_MSG=$(cat <<EOF
{}
EOF
)

echo "Migrate message: $MIGRATE_MSG"

# Query the contract state before migration
PREV_CODE_ID=$(injectived q wasm contract $CONTRACT_ADDRESS | jq -r '.contract_info.code_id')
echo "Previous code ID: $PREV_CODE_ID"

# Execute the migration transaction
MIGRATE_TXHASH=$(injectived tx wasm migrate $CONTRACT_ADDRESS $CODE_ID "$MIGRATE_MSG" --from $FROM_KEY_NAME --yes --gas-prices "160000000inj" --gas-adjustment 1.3 --gas auto -o json | jq -r '.txhash' | tr -d '"')
echo "Migrate transaction hash: $MIGRATE_TXHASH"

# Query the contract state after migration
sleep 6
NEW_CODE_ID=$(injectived q wasm contract $CONTRACT_ADDRESS | jq -r '.contract_info.code_id')
echo "New code ID: $NEW_CODE_ID"
echo "Migration successful!"


# # injectived tx wasm migrate $AUCTION_CONTRACT $CODE_ID "$MIGRATE_MSG" --from $(injectived keys show ww -a) --yes --gas-prices "160000000inj" --gas-adjustment 1.3 --gas auto -o json | jq
# injectived q wasm contract $AUCTION_CONTRACT | jq -r '.contract_info.code_id')
