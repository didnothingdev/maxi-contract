import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, Program, Wallet, web3 } from '@coral-xyz/anchor'
import { MaxiFarm, IDL as MaxiFarmIDL } from '../../target/types/maxi_farm'
import { Result, TxPassResult } from './types'
import { MaxiFarmError } from './error';
import { FEE_PRE_DIV, PROGRAMS, debug } from './constants';
import { Pdas } from './pdas';
import BN from 'bn.js';
import { calculateOutputAmount, calculateInputAmount, getMultipleAccountsInfo, getPubkeyFromStr, sleep } from './utils';
import { 
    MintLayout, 
    NATIVE_MINT_2022, 
    TOKEN_2022_PROGRAM_ID, 
    getAssociatedTokenAddressSync, 
    getMint, 
    mintTo 
} from '@solana/spl-token';
import { calcDecimalValue, calcNonDecimalValue } from './base/utils';
import { toBufferBE, toBigIntBE } from 'bigint-buffer'
import { PoolStateLayout } from './base/types';
import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { LAMPORTS_PER_SOL } from '@solana/web3.js';

const { systemProgram, tokenProgram, associatedTokenProgram } = PROGRAMS
const todo = null as any;

export type MainStateInfo = {
    owner: string,
    tradingFee: number,
    feeRecipient: string
}

export type PoolInfo = {
    owner: web3.PublicKey,
    tax: number,
    baseMint: web3.PublicKey,
    realBaseReserves: BN,
    virtBaseReserves: BN,
    realQuoteReserves: BN,
    virtQuoteReserves: BN,
    realQuoteThreshold: BN
}

export class Connectivity {
    private program: Program<MaxiFarm>
    private connection: web3.Connection
    private provider: AnchorProvider
    pdas: Pdas

    constructor(input: { walletInfo: Wallet | AnchorProvider, rpcEndPoint: string, programId: web3.PublicKey }) {
        const { programId, rpcEndPoint, walletInfo } = input
        this.connection = new web3.Connection(rpcEndPoint)
        if (walletInfo instanceof AnchorProvider) {
            this.provider = walletInfo
        } else {
            this.provider = new AnchorProvider(this.connection, walletInfo, { commitment: 'confirmed' })
        }
        this.program = new Program(MaxiFarmIDL, programId, this.provider)
        this.pdas = new Pdas(this.program.programId)
    }

    async initMainState(signer: web3.PublicKey): Promise<Result<TxPassResult>> {
        const owner = this.provider.publicKey
        if (!owner) return { Err: MaxiFarmError.WALLET_NOT_FOUND }

        const txSignature = await this.program.methods.initMainState(
            signer
        ).accounts({
            owner,
            mainState: this.pdas.mainState,
            systemProgram
        }).rpc().catch((initMainStateError) => {
            debug({ initMainStateError })
            return null
        })
        if (!txSignature) return { Err: MaxiFarmError.TX_FAILED }
        return { Ok: { txSignature } }
    }

    async transferOwnership(newOwner: web3.PublicKey): Promise<Result<TxPassResult>> {
        const owner = this.provider.publicKey
        if (!owner) return { Err: MaxiFarmError.WALLET_NOT_FOUND }

        const mainState = this.pdas.mainState
        const mainStateInfo = await this.program.account.mainState.fetch(mainState)
            .catch((fetchMainStateInfoError) => { debug({ fetchMainStateInfoError }); return null })
        if (!mainStateInfo) return { Err: MaxiFarmError.FAILED_TO_FETCH_DATA }

        const txSignature = await this.program.methods.transferOwnership(newOwner
        ).accounts({
            owner, 
            mainState: this.pdas.mainState
        }).rpc().catch(transferOwnershipError => {
            debug({ transferOwnershipError })
            return null
        })
        if (!txSignature) return { Err: MaxiFarmError.TX_FAILED }
        return { Ok: { txSignature } }
    }

    async updateMainState(input: {
        newWithdrawer?: string,
        newFeeRecipient?: string,
        quoteToken?: string,
        newTradingFee?: number,
        newTax?: number
    }): Promise<Result<TxPassResult>> {
        const owner = this.provider.publicKey
        if (!owner) return { Err: MaxiFarmError.WALLET_NOT_FOUND }

        const mainState = this.pdas.mainState
        const mainStateInfo = await this.program.account.mainState.fetch(mainState)
            .catch((fetchMainStateInfoError) => { debug({ fetchMainStateInfoError }); return null })
        if (!mainStateInfo) return { Err: MaxiFarmError.FAILED_TO_FETCH_DATA }

        let newWithdrawer: null | web3.PublicKey = null
        let newFeeRecipient: null | web3.PublicKey = null
        let newTradingFee: null | BN = null
        let newTax: null | BN = null
        
        if (input.newWithdrawer) {
            const address = getPubkeyFromStr(input.newWithdrawer)
            if (!address) return { Err: MaxiFarmError.INVALID_INPUT }
            newWithdrawer = address
        } else {
            newWithdrawer = mainStateInfo.withdrawer
        }

        if (input.newTax) {
            const tmpTax = Math.trunc(input.newTax * 100)
            newTax = new BN(tmpTax)
        } else {
            newTax = mainStateInfo.tax
        }

        if (input.newTradingFee) {
            const tmpFee = Math.trunc(input.newTradingFee * FEE_PRE_DIV)
            newTradingFee = new BN(tmpFee)
        } else {
            newTradingFee = mainStateInfo.tradingFee
        }

        if (input.newFeeRecipient) {
            const address = getPubkeyFromStr(input.newFeeRecipient)
            if (!address) return { Err: MaxiFarmError.INVALID_INPUT }
            newFeeRecipient = address
        } else {
            newFeeRecipient = mainStateInfo.feeRecipient
        }

        const txSignature = await this.program.methods.updateMainState({
            withdrawer: newWithdrawer,
            tax: newTax,
            tradingFee: newTradingFee,
            feeRecipient: newFeeRecipient
        }).accounts({
            owner, 
            mainState: this.pdas.mainState
        }).rpc().catch(updateMainStateError => {
            debug({ updateMainStateError })
            return null
        })
        if (!txSignature) return { Err: MaxiFarmError.TX_FAILED }
        return { Ok: { txSignature } }
    }

    async setReferrer(input: { referrer: string }): Promise<Result<TxPassResult>> {
        const user = this.provider.publicKey
        if (!user) return { Err: MaxiFarmError.WALLET_NOT_FOUND }
        const referrer = getPubkeyFromStr(input.referrer)
        const referralAccount = this.pdas.getReferralStateAccount({ owner: user })
        const referrerAccount = this.pdas.getReferralStateAccount({ owner: referrer })

        const txSignature = await this.program.methods.registerUser(referrer
        ).accounts({
            user,
            referralAccount: referralAccount,
            referrerAccount: referrerAccount,
            systemProgram
        }).rpc().catch(setReferrerError => {
            debug({ setReferrerError })
            return null
        })
        if (!txSignature) return { Err: MaxiFarmError.TX_FAILED }
        return { Ok: { txSignature }}
    }

    async createPool(input: { baseToken: string, metadataUri: string, tax: number, maxFeeTokens: number, realQuoteThreshold: number, privSalePeriod?: number }): Promise<Result<TxPassResult & { poolId: string }>> {
        const creator = this.provider.publicKey
        if (!creator) return { Err: MaxiFarmError.WALLET_NOT_FOUND }
        const baseMint = getPubkeyFromStr(input.baseToken)
        if (!baseMint) return { Err: MaxiFarmError.INVALID_INPUT }
        const poolState = this.pdas.getPoolStateAccount({ baseMint, owner: creator })
        const reserverBaseAta = getAssociatedTokenAddressSync(baseMint, poolState, true, tokenProgram)
        
        const txSignature = await this.program.methods.createPool(
            input.metadataUri, 
            new BN(input.tax), 
            new BN(input.maxFeeTokens), 
            new BN(input.realQuoteThreshold),
            input.privSalePeriod ? new BN(input.privSalePeriod) : null
        ).accounts({
            creator: creator,
            mainState: this.pdas.mainState,
            poolState,
            baseMint,
            reserverBaseAta,
            associatedTokenProgram,
            tokenProgram,
            systemProgram
        }).preInstructions([
            web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 300_000 })
        ]).rpc().catch(createPoolError => {
            debug({ createPoolError })
            return null
        })
        if (!txSignature) return { Err: MaxiFarmError.TX_FAILED }
        return { Ok: { txSignature, poolId: poolState.toBase58() } }
    }

    async buy(input: { amount: number, poolId: string }): Promise<Result<TxPassResult>> {
        const buyer = this.provider.publicKey
        if (!buyer) return { Err: MaxiFarmError.WALLET_NOT_FOUND }
        const poolState = getPubkeyFromStr(input.poolId)
        if (!poolState) return { Err: MaxiFarmError.INVALID_INPUT }
        const mainStateInfo = await this.program.account.mainState.fetch(this.pdas.mainState)
            .catch((fetchMainStateInfoError) => { debug({ fetchMainStateInfoError }); return null })
        if (!mainStateInfo) return { Err: MaxiFarmError.FAILED_TO_FETCH_DATA }
        const poolInfo = await this.program.account.poolState.fetch(poolState)
            .catch((fetchPoolInfoError) => { debug({ fetchPoolInfoError }); return null })
        if (!poolInfo) return { Err: MaxiFarmError.POOL_NOT_FOUND }
        const { baseMint } = poolInfo
        const amount = new BN(toBufferBE(BigInt(calcNonDecimalValue(input.amount, 9).toString()), 8))
        const buyerBaseAta = getAssociatedTokenAddressSync(baseMint, buyer, true, tokenProgram)
        const reserverBaseAta = getAssociatedTokenAddressSync(baseMint, poolState, true, tokenProgram)

        const referralAccount = this.pdas.getReferralStateAccount({ owner: buyer })
        const referralState = await this.program.account.referralState.fetch(referralAccount)
            .catch((fetchReferralStateError) => { debug({ fetchReferralStateError }); return null })
        console.log('referralAccount:', referralAccount)
        console.log('referralState:', referralState)

        const {nextReferralAccount: tier1ReferralAccount, nextReferralState: tier1ReferralState} = await this.getNextReferralInfo(referralState)
        console.log('tier1ReferralAccount:', tier1ReferralAccount)
        console.log('tier1ReferralState:', tier1ReferralState)

        const {nextReferralAccount: tier2ReferralAccount, nextReferralState: tier2ReferralState} = await this.getNextReferralInfo(tier1ReferralState)
        console.log('tier2ReferralAccount:', tier2ReferralAccount)
        console.log('tier2ReferralState:', tier2ReferralState)

        const {nextReferralAccount: tier3ReferralAccount, /* nextReferralState: tier3ReferralState */} = await this.getNextReferralInfo(tier2ReferralState)
        console.log('tier3ReferralAccount:', tier3ReferralAccount)
        
        const txSignature = await this.program.methods.buyTokensFromExactSol(
            amount, 
            new BN(0), 
            null
        ).accounts({
            buyer,
            mainState: this.pdas.mainState,
            feeRecipient: mainStateInfo.feeRecipient,
            poolState,
            baseMint,
            buyerBaseAta,
            reserverBaseAta,
            tier1Referral: tier1ReferralAccount,
            tier2Referral: tier2ReferralAccount,
            tier3Referral: tier3ReferralAccount,
            ixSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
            associatedTokenProgram,
            tokenProgram,
            systemProgram
        }).preInstructions([
            web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 300_000 })
        ]).rpc().catch(buyTxError => {
            debug({ buyTxError })
            return null
        })
        if (!txSignature) return { Err: MaxiFarmError.TX_FAILED }
        return { Ok: { txSignature } }
    }

    async buy2(input: { amount: number, poolId: string }): Promise<Result<TxPassResult>> {
        const buyer = this.provider.publicKey
        if (!buyer) return { Err: MaxiFarmError.WALLET_NOT_FOUND }
        const poolState = getPubkeyFromStr(input.poolId)
        if (!poolState) return { Err: MaxiFarmError.INVALID_INPUT }
        const mainStateInfo = await this.program.account.mainState.fetch(this.pdas.mainState)
            .catch((fetchMainStateInfoError) => { debug({ fetchMainStateInfoError }); return null })
        if (!mainStateInfo) return { Err: MaxiFarmError.FAILED_TO_FETCH_DATA }
        const poolInfo = await this.program.account.poolState.fetch(poolState)
            .catch((fetchPoolInfoError) => { debug({ fetchPoolInfoError }); return null })
        if (!poolInfo) return { Err: MaxiFarmError.POOL_NOT_FOUND }
        const { baseMint } = poolInfo
        const amount = new BN(toBufferBE(BigInt(calcNonDecimalValue(input.amount * 5 / 4, 6).toString()), 8))
        const buyerBaseAta = getAssociatedTokenAddressSync(baseMint, buyer, true, tokenProgram)
        const reserverBaseAta = getAssociatedTokenAddressSync(baseMint, poolState, true, tokenProgram)
        const result = await this.getInputAmountOnBuy({ outputAmount: input.amount, poolId: input.poolId })
        if (!result) return { Err: MaxiFarmError.INVALID_INPUT }
        const maxQuote = (result.Ok || 0 ) * 11 / 10
        const maxQuoteAmount = maxQuote * LAMPORTS_PER_SOL

        const referralAccount = this.pdas.getReferralStateAccount({ owner: buyer })
        const referralState = await this.program.account.referralState.fetch(referralAccount)
            .catch((fetchReferralStateError) => { debug({ fetchReferralStateError }); return null })
        console.log('referralAccount:', referralAccount)
        console.log('referralState:', referralState)

        const {nextReferralAccount: tier1ReferralAccount, nextReferralState: tier1ReferralState} = await this.getNextReferralInfo(referralState)
        console.log('tier1ReferralAccount:', tier1ReferralAccount)
        console.log('tier1ReferralState:', tier1ReferralState)

        const {nextReferralAccount: tier2ReferralAccount, nextReferralState: tier2ReferralState} = await this.getNextReferralInfo(tier1ReferralState)
        console.log('tier2ReferralAccount:', tier2ReferralAccount)
        console.log('tier2ReferralState:', tier2ReferralState)

        const {nextReferralAccount: tier3ReferralAccount, /* nextReferralState: tier3ReferralState */} = await this.getNextReferralInfo(tier2ReferralState)
        console.log('tier3ReferralAccount:', tier3ReferralAccount)

        const txSignature = await this.program.methods.buyExactTokensFromSol(
            amount, 
            new BN(maxQuoteAmount), 
            null
        ).accounts({
            buyer,
            mainState: this.pdas.mainState,
            feeRecipient: mainStateInfo.feeRecipient,
            poolState,
            baseMint,
            buyerBaseAta,
            reserverBaseAta,
            tier1Referral: tier1ReferralAccount,
            tier2Referral: tier2ReferralAccount,
            tier3Referral: tier3ReferralAccount,
            ixSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
            associatedTokenProgram,
            tokenProgram,
            systemProgram
        }).preInstructions([
            web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 300_000 })
        ]).rpc().catch(buyTxError => {
            debug({ buyTxError })
            return null
        })
        if (!txSignature) return { Err: MaxiFarmError.TX_FAILED }
        return { Ok: { txSignature } }
    }

    async sell(input: { amount: number, poolId: string }): Promise<Result<TxPassResult>> {
        const seller = this.provider.publicKey
        if (!seller) return { Err: MaxiFarmError.WALLET_NOT_FOUND }
        const poolState = getPubkeyFromStr(input.poolId)
        if (!poolState) return { Err: MaxiFarmError.INVALID_INPUT }
        const mainStateInfo = await this.program.account.mainState.fetch(this.pdas.mainState)
            .catch((fetchMainStateInfoError) => { debug({ fetchMainStateInfoError }); return null })
        if (!mainStateInfo) return { Err: MaxiFarmError.FAILED_TO_FETCH_DATA }
        const poolInfo = await this.program.account.poolState.fetch(poolState)
            .catch((fetchPoolInfoError) => { debug({ fetchPoolInfoError }); return null })
        if (!poolInfo) return { Err: MaxiFarmError.POOL_NOT_FOUND }
        const { baseMint } = poolInfo;
        const baseMintDecimals = 6
        const sellAmount = new BN(toBufferBE(BigInt(calcNonDecimalValue(input.amount, baseMintDecimals).toString()), 8))
        const sellerBaseAta = getAssociatedTokenAddressSync(baseMint, seller, true, tokenProgram)
        const reserverBaseAta = getAssociatedTokenAddressSync(baseMint, poolState, true, tokenProgram)

        const referralAccount = this.pdas.getReferralStateAccount({ owner: seller })
        const referralState = await this.program.account.referralState.fetch(referralAccount)
            .catch((fetchReferralStateError) => { debug({ fetchReferralStateError }); return null })
        console.log('referralAccount:', referralAccount)
        console.log('referralState:', referralState)

        const {nextReferralAccount: tier1ReferralAccount, nextReferralState: tier1ReferralState} = await this.getNextReferralInfo(referralState)
        console.log('tier1ReferralAccount:', tier1ReferralAccount)
        console.log('tier1ReferralState:', tier1ReferralState)

        const {nextReferralAccount: tier2ReferralAccount, nextReferralState: tier2ReferralState} = await this.getNextReferralInfo(tier1ReferralState)
        console.log('tier2ReferralAccount:', tier2ReferralAccount)
        console.log('tier2ReferralState:', tier2ReferralState)

        const {nextReferralAccount: tier3ReferralAccount, /* nextReferralState: tier3ReferralState */} = await this.getNextReferralInfo(tier2ReferralState)
        console.log('tier3ReferralAccount:', tier3ReferralAccount)
        
        const txSignature = await this.program.methods.sell(
            sellAmount, 
            new BN(0), 
            null
        ).accounts({
            seller,
            mainState: this.pdas.mainState,
            feeRecipient: mainStateInfo.feeRecipient,
            poolState,
            baseMint,
            sellerBaseAta,
            reserverBaseAta,
            tier1Referral: tier1ReferralAccount,
            tier2Referral: tier2ReferralAccount,
            tier3Referral: tier3ReferralAccount,
            ixSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
            associatedTokenProgram,
            tokenProgram,
            systemProgram
        }).preInstructions([
            web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 300_000 })
        ]).rpc().catch(sellTxError => {
            debug({ sellTxError })
            return null
        })
        if (!txSignature) return { Err: MaxiFarmError.TX_FAILED }
        return { Ok: { txSignature } }
    }

    async withdraw(input: { poolId: string }): Promise<Result<TxPassResult>> {
        const withdrawer = this.provider.publicKey
        if (!withdrawer) return { Err: MaxiFarmError.WALLET_NOT_FOUND }

        const mainState = this.pdas.mainState
        const mainStateInfo = await this.program.account.mainState.fetch(mainState)
            .catch((fetchMainStateInfoError) => { debug({ fetchMainStateInfoError }); return null })
        if (!mainStateInfo) return { Err: MaxiFarmError.FAILED_TO_FETCH_DATA }

        const poolState = getPubkeyFromStr(input.poolId)
        if (!poolState) return { Err: MaxiFarmError.INVALID_INPUT }
        const poolInfo = await this.program.account.poolState.fetch(poolState)
            .catch((fetchPoolInfoError) => { debug({ fetchPoolInfoError }); return null })
        if (!poolInfo) return { Err: MaxiFarmError.POOL_NOT_FOUND }
        const { baseMint } = poolInfo

        const reserverBaseAta = getAssociatedTokenAddressSync(baseMint, poolState, true, tokenProgram)
        const withdrawerBaseAta = getAssociatedTokenAddressSync(baseMint, withdrawer, true, tokenProgram)

        const txSignature = await this.program.methods.withdraw(
        ).accounts({
            withdrawer,
            mainState, poolState,
            baseMint,
            reserverBaseAta,
            withdrawerBaseAta,
            associatedTokenProgram,
            tokenProgram,
            systemProgram
        }).rpc().catch((collectTradingFeeError) => debug({ collectTradingFeeError }))
        if (!txSignature) return { Err: MaxiFarmError.TX_FAILED }
        return { Ok: { txSignature } }
    }

    async getMainStateInfo(): Promise<MainStateInfo | null> {
        const mainState = this.pdas.mainState
        const mainStateInfo = await this.program.account.mainState.fetch(mainState).catch(fetchMainStateError => {
            debug({ fetchMainStateError })
            return null
        })
        if (!mainStateInfo) return null
        const tradingFee = mainStateInfo.tradingFee.toNumber() / FEE_PRE_DIV
        return {
            owner: mainStateInfo.owner.toBase58(),
            tradingFee,
            feeRecipient: mainStateInfo.feeRecipient.toBase58()
        }
    }

    async getPoolInfo(poolIdStr: string): Promise<PoolInfo | null> {
        const poolId = getPubkeyFromStr(poolIdStr)
        if (!poolId) {
            debug("Invalid pool key")
            return null
        }
        const poolInfo = await this.program.account.poolState.fetch(poolId).catch(fetchPoolInfoError => {
            debug({ fetchPoolInfoError })
            return null
        })
        if (!poolInfo) return null
        const { baseMint, virtBaseReserves, realBaseReserves, virtQuoteReserves, realQuoteReserves, realQuoteThreshold, owner } = poolInfo
        return {
            baseMint, 
            tax: Number(poolInfo.tax) / 100, 
            virtBaseReserves, realBaseReserves, virtQuoteReserves, realQuoteReserves, realQuoteThreshold, 
            owner
        }
    }

    async getNextReferralInfo(referralState: any): Promise<{nextReferralAccount: any, nextReferralState: any}> {
        let nextReferralAccount;
        let nextReferralState;

        if (!referralState || !referralState.referrer) {
            nextReferralAccount = null
            nextReferralState = null
        } else {
            nextReferralAccount = this.pdas.getReferralStateAccount({ owner: referralState.referrer })
            nextReferralState = await this.program.account.referralState.fetch(nextReferralAccount)
                .catch((fetchReferralStateError) => { debug({ fetchReferralStateError }); return null })
            if (!nextReferralState)
                nextReferralAccount = null
        }

        return {nextReferralAccount, nextReferralState}
    }

    async getOutputAmountOnBuy(input: { inputAmount: number, poolId: string }): Promise<Result<number>> {
        const mainState = await this.getMainStateInfo()
        if (!mainState) return { Err: MaxiFarmError.MAIN_STATE_INFO_NOT_FOUND }
        const poolInfo = await this.getPoolInfo(input.poolId)
        if (!poolInfo) return { Err: MaxiFarmError.POOL_NOT_FOUND }
        // console.log(`poolInfo.virtBaseReserves: ${poolInfo.virtBaseReserves}`)
        // console.log(`poolInfo.realBaseReserves: ${poolInfo.realBaseReserves}`)
        // console.log(`poolInfo.virtQuoteReserves: ${poolInfo.virtQuoteReserves}`)
        // console.log(`poolInfo.realQuoteReserves: ${poolInfo.realQuoteReserves}`)
        // console.log(`poolInfo.realQuoteThreshold: ${poolInfo.realQuoteThreshold}`)
        const fee = input.inputAmount * mainState.tradingFee / 100
        let inputAmount = Math.trunc((input.inputAmount - fee) * LAMPORTS_PER_SOL)
        // console.log(`input.inputAmount: ${input.inputAmount}, fee: ${fee}, inputAmount: ${inputAmount}`)
        
        if (Number(poolInfo.realQuoteReserves) + inputAmount > Number(poolInfo.realQuoteThreshold))
            inputAmount = Number(poolInfo.realQuoteThreshold) - Number(poolInfo.realQuoteReserves)
        const quoteReserves = poolInfo.realQuoteReserves.add(poolInfo.virtQuoteReserves)
        const inputReserve = Number(quoteReserves.toString())
        const baseReserves = poolInfo.realBaseReserves.add(poolInfo.virtBaseReserves)
        const outputReserve = Number(baseReserves.toString())
        const outputAmount = Math.trunc(calculateOutputAmount({ inputAmount, inputReserve, outputReserve }) * 4 / 5); // 20% tax
        const decimals = 6
        return {
            Ok: calcDecimalValue(outputAmount, decimals)
        }
    }

    async getInputAmountOnBuy(input: { outputAmount: number, poolId: string }): Promise<Result<number>> {
        const mainState = await this.getMainStateInfo()
        if (!mainState) return { Err: MaxiFarmError.MAIN_STATE_INFO_NOT_FOUND }
        const poolInfo = await this.getPoolInfo(input.poolId)
        if (!poolInfo) return { Err: MaxiFarmError.POOL_NOT_FOUND }
        const decimals = 6
        const outputAmount = calcNonDecimalValue(input.outputAmount * 5 / 4, decimals) // compensate 20% tax
        const quoteReserves = poolInfo.realQuoteReserves.add(poolInfo.virtQuoteReserves)
        const inputReserve = Number(quoteReserves.toString())
        const baseReserves = poolInfo.realBaseReserves.add(poolInfo.virtBaseReserves)
        const outputReserve = Number(baseReserves.toString())
        const inputAmount_ = calculateInputAmount({ outputAmount, inputReserve, outputReserve })
        const fee = Math.trunc(inputAmount_ * mainState.tradingFee / (100 - mainState.tradingFee))
        const inputAmount = inputAmount_ + fee
        return {
            Ok: calcDecimalValue(inputAmount, 9)
        }
    }

    async getOutputAmountOnSell(input: { inputAmount: number, poolId: string }): Promise<Result<number>> {
        const mainState = await this.getMainStateInfo();
        if (!mainState) return { Err: MaxiFarmError.MAIN_STATE_INFO_NOT_FOUND }
        const poolInfo = await this.getPoolInfo(input.poolId)
        if (!poolInfo) return { Err: MaxiFarmError.POOL_NOT_FOUND }
        const decimals = 6
        const inputAmount = calcNonDecimalValue(input.inputAmount * 4 / 5, decimals) // 20% tax
        const baseReserves = poolInfo.realBaseReserves.add(poolInfo.virtBaseReserves)
        const inputReserve = Number(baseReserves.toString())
        const quoteReserves = poolInfo.realQuoteReserves.add(poolInfo.virtQuoteReserves)
        const outputReserve = Number(quoteReserves.toString())
        const _outputAmount = calculateOutputAmount({ inputAmount, inputReserve, outputReserve })
        const fee = Math.trunc(_outputAmount * mainState.tradingFee / 100)
        const outputAmount = _outputAmount - fee
        return {
            Ok: calcDecimalValue(outputAmount, 9)
        }
    }
}