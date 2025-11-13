// test.js
require('dotenv').config({ override: true }); // allows .env to override system envs
const { readFileSync } = require('fs');

// ---- Polyfill fetch for Node 16 ----
const { fetch, Headers, Request, Response } = require('undici');
globalThis.fetch = fetch;
globalThis.Headers = Headers;
globalThis.Request = Request;
globalThis.Response = Response;

const { createPublicClient, http, isAddress } = require('viem');

// ---- Config ----
const RPC = process.env.RPC_URL || 'https://sepolia-rollup.arbitrum.io/rpc';
const CONTRACT = (process.env.CONTRACT || '').toLowerCase();
const USER = process.env.USER_ADDR; // Use USER_ADDR instead of USER

// ---- ABI ----
const abi = JSON.parse(readFileSync('./abi.clean.json', 'utf8'));

// ---- Public client (for read-only calls) ----
const client = createPublicClient({
  transport: http(RPC),
});

// ---- Helper functions ----
const str = (x) => (typeof x === 'bigint' ? x.toString() : String(x));

function assertHexAddr(a, label = 'address') {
  if (!a || !isAddress(a)) {
    throw new Error(`${label} "${a}" is invalid. Expected 0x + 40 hex chars.`);
  }
}

// ---- Read functions ----
async function readCharityPool() {
  const res = await client.readContract({
    address: CONTRACT,
    abi,
    functionName: 'charityPoolTotal',
    args: [],
  });
  console.log('charityPoolTotal =', str(res));
}

async function readBalancesOf(addr) {
  assertHexAddr(addr, 'USER_ADDR');
  const [available, staked, earned, burned] = await client.readContract({
    address: CONTRACT,
    abi,
    functionName: 'balancesOf',
    args: [addr],
  });
  console.log('balancesOf', addr, {
    available: str(available),
    staked: str(staked),
    earned: str(earned),
    burned: str(burned),
  });
}

// ---- Main Execution ----
(async () => {
  try {
    assertHexAddr(CONTRACT, 'CONTRACT');
    console.log('Contract:', CONTRACT);
    console.log('RPC:', RPC);
    console.log('---');

    await readCharityPool();

    if (USER) {
      await readBalancesOf(USER);
    } else {
      console.log('Tip: set USER_ADDR in .env to read your balances.');
    }
  } catch (e) {
    console.error('Error:', e.message || e);
  }
})();
