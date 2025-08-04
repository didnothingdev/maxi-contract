import { web3 } from "@coral-xyz/anchor";
import { debug } from "./constants";

export async function sleep(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export function getPubkeyFromStr(key: string) {
    try {
        return new web3.PublicKey(key)
    } catch (pubkeyParseError) {
        debug({ pubkeyParseError })
        return null
    }
}

export async function getMultipleAccountsInfo(connection: web3.Connection, pubkeys: web3.PublicKey[], opt?: { retry?: boolean, duration?: number }) {
    opt = opt ?? {}
    opt.retry = opt.retry ?? true
    opt.duration = opt.duration ?? 2000
    const { duration, retry } = opt
    const res = await connection.getMultipleAccountsInfo(pubkeys).catch(async () => {
        if (retry) {
            await sleep(duration)
            return await connection.getMultipleAccountsInfo(pubkeys).catch(getMultipleAccountsInfoError => {
                debug({ getMultipleAccountsInfoError })
                return null
            })
        }
        return null
    })
    return res
}

// output_amount = output_reserve * input_amount / (input_reserve + input_amount)
export function calculateOutputAmount({ inputAmount, inputReserve, outputReserve }: { inputAmount: number, inputReserve: number, outputReserve: number }) {
    console.log(`calculateOutputAmount - inputAmount: ${inputAmount}, inputReserve: ${inputReserve}, outputReserve: ${outputReserve}`)
    const amount = outputReserve * inputAmount
    const divider = inputReserve + inputAmount
    return Math.trunc(amount / divider)
}

// input_amount = output_amount * input_reserve / (output_reserve - output_amount)
export function calculateInputAmount({ outputAmount, inputReserve, outputReserve }: { outputAmount: number, inputReserve: number, outputReserve: number }) {
    console.log(`calculateInputAmount - outputAmount: ${outputAmount}, inputReserve: ${inputReserve}, outputReserve: ${outputReserve}`)
    if (outputAmount >= outputReserve) {
        throw new Error(`outputAmount can't be greater than or equal to outputReserve`)
    }

    const amount = inputReserve * outputAmount
    const divider = outputReserve - outputAmount
    return Math.trunc(amount / divider)
}
