// write.js
require('dotenv').config({ override: true });
const { readFileSync } = require('fs');
const { fetch, Headers, Request, Response } = require('undici');

// Polyfill fetch for Node 16+
globalThis.fetch = fetch;
globalThis.Headers = Headers;
globalThis.Request = Request;
globalThis.Response = Response;

// --- Imports ---
const {
  createWalletClient,
  createPublicClient,
  http,
  parseEther,
  waitForTransactionReceipt,
} = require('viem');
const { privateKeyToAccount } = require('viem/accounts');  // ✅ fixed import
const { arbitrumSepolia } = require('viem/chains');

// ---- Config ----
const RPC = process.env.RPC_URL || 'https://sepolia-rollup.arbitrum.io/rpc';
const PRIVATE_KEY = process.env.PRIVATE_KEY;
const CONTRACT = (process.env.CONTRACT || '').toLowerCase();

// ---- ABI ----
const abi = JSON.parse(readFileSync('./abi.clean.json', 'utf8'));

// ---- Wallet setup ----
const account = privateKeyToAccount(`0x${PRIVATE_KEY.replace(/^0x/, '')}`);

const client = createWalletClient({
  account,
  chain: arbitrumSepolia,
  transport: http(RPC),
});

const publicClient = createPublicClient({
  chain: arbitrumSepolia,
  transport: http(RPC),
});

// ---- Helper ----
const str = (x) => (typeof x === 'bigint' ? x.toString() : String(x));

// ---- Main ----
(async () => {
  try {
    const amount = process.argv[2];
    if (!amount) {
      console.error('❌ Usage: node write.js <amount>');
      process.exit(1);
    }

    console.log(`🚀 Depositing ${amount} credits to ${CONTRACT}...`);

    // Send the transaction
    const hash = await client.writeContract({
      address: CONTRACT,
      abi,
      functionName: 'depositCredits',
      args: [BigInt(amount)],
      account,
    });

    console.log('⏳ Transaction sent:', hash);

    // Wait for confirmation
    const receipt = await publicClient.waitForTransactionReceipt({ hash });
    console.log('✅ Tx confirmed in block', receipt.blockNumber);
  } catch (err) {
    console.error('Error:', err.message || err);
  }
})();
