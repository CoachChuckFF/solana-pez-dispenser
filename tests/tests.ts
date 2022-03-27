import * as anchor from "@project-serum/anchor";
import * as helpers from "@coach-chuck/solana-helpers";
import * as pez from "../ts/pez"
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";

const secretArray = require('/Users/drkrueger/.config/solana/programs/pez.json');
const secret = new Uint8Array(secretArray);
const payerKeypair = anchor.web3.Keypair.fromSecretKey(secret);

const sleep = (ms: number) => {
    return new Promise((resolve, reject) => {
        setTimeout(()=>{
            resolve(null);
        }, ms);
    });
}

const main = async() => {
    console.log("🚀 Starting test...")
  
    let ownerWallet = new NodeWallet(payerKeypair);
    const provider = helpers.getSolanaProvider(ownerWallet);
    anchor.setProvider(provider);
    
    console.log("Creating SFT...");
    let SFT = await helpers.createSPL(
      provider,
      1000000
    );
  
    console.log("Creating provider...");
    let pezProvider = await pez.PezProvider.init(provider);
  
    console.log("Calling create...");
    let pezDispenser = await pez.createPezDispenser(
        pezProvider,
        SFT,
        undefined,
        new anchor.BN(500),
    );
  
    console.log(pezDispenser);
  
    console.log("Updating Pez...");
    pezDispenser = await pez.updatePezDispenser(
        pezProvider,
        pezDispenser,
        "Tod",
        new anchor.BN(0),
        new anchor.BN(0),
    );
    console.log(pezDispenser);

    console.log("Loading Pez...");
    pezDispenser = await pez.loadPezDispenser(
        pezProvider,
        pezDispenser,
        new anchor.BN(500),
    );
    console.log(pezDispenser);


    console.log("Emptying Pez...");
    pezDispenser = await pez.emptyPezDispenser(
        pezProvider,
        pezDispenser,
        new anchor.BN(100),
    );
    console.log(pezDispenser);

    let pezCandy = await pez.getPezCandyAccount(
        pezProvider,
        pezDispenser,
    );

    let ownerCandy = await pez.getOwnerCandyAccount(
        pezProvider,
        pezDispenser,
    );

    console.log("Pez: " + pezCandy.amount.toNumber());
    console.log("Owner: " + ownerCandy.amount.toNumber());

    console.log("Loading the rest");
    pezDispenser = await pez.loadPezDispenser(
        pezProvider,
        pezDispenser,
    );
    console.log(pezDispenser);

    console.log("Taking Pez...");
    pezDispenser = await pez.takePez(
        pezProvider,
        pezDispenser,
    );
    console.log(pezDispenser);

    pezCandy = await pez.getPezCandyAccount(
        pezProvider,
        pezDispenser,
    );

    ownerCandy = await pez.getOwnerCandyAccount(
        pezProvider,
        pezDispenser,
    );

    console.log("Pez: " + pezCandy.amount.toNumber());
    console.log("Owner: " + ownerCandy.amount.toNumber());
  
    console.log("... to the moon! 🌑")
  }
  
  const runMain = async () => {
    try {
      await main();
      process.exit(0);
    } catch (error) {
      console.error(error);
      process.exit(1);
    }
  };
  
  runMain();


