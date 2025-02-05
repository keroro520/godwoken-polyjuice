#ifndef POLYJUICE_GLOBALS_H
#define POLYJUICE_GLOBALS_H

#define POLYJUICE_VERSION "v1.1.3-beta"

#define ETH_ADDRESS_LEN 20

/* Key type for ETH Address Registry */
#define GW_ACCOUNT_SCRIPT_HASH_TO_ETH_ADDR 200
#define ETH_ADDR_TO_GW_ACCOUNT_SCRIPT_HASH 201

/** Polyjuice contract account (normal/create2) script args size */
#define CONTRACT_ACCOUNT_SCRIPT_ARGS_LEN 56 /* 32 + 4 + 20 */
#define CREATOR_SCRIPT_ARGS_LEN 36			/* 32 + 4 */

static uint8_t g_rollup_script_hash[32] = {0};
static uint32_t g_sudt_id = UINT32_MAX;

/**
 * Receipt.contractAddress is the created contract,
 * if the transaction was a contract creation, otherwise null
 */
static uint8_t g_created_address[20] = {0};
static uint32_t g_created_id = UINT32_MAX;

/**
 * @brief chain_id in Godwoken RollupConfig
 */
static uint64_t g_chain_id = UINT64_MAX;
/**
 * creator_account, known as root account
 * @see https://github.com/nervosnetwork/godwoken/blob/develop/docs/life_of_a_polyjuice_transaction.md#root-account--deployment
 */
static uint32_t g_creator_account_id = UINT32_MAX;

static evmc_address g_tx_origin = {0};

static uint8_t g_script_code_hash[32] = {0};
static uint8_t g_script_hash_type = 0xff;

#endif // POLYJUICE_GLOBALS_H
