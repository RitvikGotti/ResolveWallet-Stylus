
require('dotenv').config({ override: true });
const { readFileSync } = require('fs');
const { fetch, Headers, Request, Response } = require('undici');

globalThis.fetch = fetch;
globalThis.Headers = Headers;
globalThis.Request = Request;
globalThis.Response = Response;

const {
  createWalletClient,
  http,
  parseEther,
} = require('viem');
const { privateKeyToAccount } = require('viem/accounts');


const RPC = process.env.RPC_URL || 'https://sepolia-rollup.arbitrum.io/rpc';
const CONTRACT = (process.env.CONTRACT || '').toLowerCase();
const PRIVATE_KEY = process.env.PRIVATE_KEY;


const abi = JSON.parse(readFileSync('./abi.clean.json', 'utf8'));


function str(x) {
  return typeof x === 'bigint' ? x.toString() : String(x);
}

if (!PRIVATE_KEY) {
  console.error('Error: PRIVATE_KEY missing in .env');
  process.exit(1);
}

const account = privateKeyToAccount(`0x${PRIVATE_KEY.replace(/^0x/, '')}`);

const client = createWalletClient({
  account,
  transport: http(RPC),
});


async function send(functionName, args) {
  console.log(`Calling ${functionName}(${args.map(str).join(', ')})...`);

  const hash = await client.writeContract({
    address: CONTRACT,
    abi,
    functionName,
    args,
  });

  console.log('Tx hash:', hash);
  console.log(
    'View on Arbiscan:',
    `https://sepolia.arbiscan.io/tx/${hash}`
  );
}



(async () => {
  try {
    const cmd = process.argv[2];
    const value = process.argv[3];

    if (!cmd) {
      console.log('Usage:');
      console.log('  node write.js deposit 100');
      console.log('  node write.js complete 1');
      console.log('  node write.js miss 1');
      process.exit(0);
    }

    if (cmd === 'deposit') {
      if (!value) throw new Error('Missing amount');
      const amount = BigInt(value);
      await send('depositCredits', [amount]);
    } else if (cmd === 'complete') {
      if (!value) throw new Error('Missing goal id');
      const goalId = BigInt(value);
      await send('completeGoal', [goalId]);
    } else if (cmd === 'miss') {
      if (!value) throw new Error('Missing goal id');
      const goalId = BigInt(value);
      await send('missGoal', [goalId]);
    } else {
      throw new Error(`Unknown command "${cmd}". Use deposit|complete|miss.`);
    }
  } catch (e) {
    console.error('Error:', e.message || e);
  }
})();
