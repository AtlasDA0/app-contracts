import codegen from '@cosmwasm/ts-codegen';

codegen({
  contracts: [
    {
      name: 'Raffle',
      dir: '../contracts/raffles/schema'
    },
    {
      name: 'NFTLoan',
      dir: '../contracts/nft-loan/schema'
    }
  ],
  outPath: './src/',

  // options are completely optional ;)
  options: {
    bundle: {
      bundleFile: 'bundle.ts',
      scope: 'contracts'
    },
    types: {
      enabled: true
    },
    client: {
      enabled: true
    },
    reactQuery: {
      enabled: false,
      optionalClient: true,
      version: 'v4',
      mutations: true,
      queryKeys: true,
      queryFactory: true,
    },
    recoil: {
      enabled: false
    },
    messageComposer: {
        enabled: true
    },
  }
}).then(() => {
  console.log('✨ all done!');
});