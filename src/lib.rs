mod result_ext;

use ethabi::{
    token::{LenientTokenizer, Token, Tokenizer},
    Contract, Error, ParamType,
};
use neon::{prelude::*, result::Throw};
use result_ext::ResultExt;
use std::str;

/**
 * Wraps ethabi `LenientTokenizer` functions so that they can
 * be used via JavaScript.
 * 
 * This example was very helpful:
 * https://github.com/neon-bindings/examples/tree/223ba9b4870fefc18ad18a7a474f49acbe8b77f6/examples/async-sqlite
 */
pub struct Coder(Contract);

impl Finalize for Coder {}

// Internal Implementation
impl Coder {
    pub fn new(abi_json: &str) -> anyhow::Result<Self> {
        let contract = Contract::load(abi_json.as_bytes()).map(Self)?;
        Ok(contract)
    }

    pub fn argument_types(&self, function: &str) -> anyhow::Result<Vec<ParamType>> {
        let function = self.0.function(function)?;
        let result = function
            .inputs
            .iter()
            .map(|param| param.kind.clone())
            .collect();
        Ok(result)
    }

    pub fn encode_input(&self, function: &str, arguments: &[Token]) -> anyhow::Result<String> {
        let function = self.0.function(function)?;
        let result = function.encode_input(arguments).map(hex::encode)?;
        Ok(result)
    }

    pub fn decode_input(&self, function: &str, data: &str) -> anyhow::Result<Vec<Token>> {
        let function = self.0.function(function)?;
        let data = hex::decode(remove_bytes4(data))?;
        Ok(function.decode_input(&data)?)
    }

    pub fn decode_output(&self, function: &str, data: &str) -> anyhow::Result<Vec<Token>> {
        let function = self.0.function(function)?;
        let data = hex::decode(remove_hex_prefix(data))?;
        Ok(function.decode_output(&data)?)
    }
}

// Implementation exposed to TS
impl Coder {
    // Create a new instance of `Database` and place it inside a `JsBox`
    // JavaScript can hold a reference to a `JsBox`, but the contents are opaque
    fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<Coder>> {
        let abi = cx.argument::<JsString>(0)?.value(&mut cx);
        let coder = Coder::new(abi.as_ref()).unwrap();

        Ok(cx.boxed(coder))
    }

    fn js_encode_input(mut cx: FunctionContext) -> JsResult<JsString> {
        let function = cx.argument::<JsString>(0)?.value(&mut cx);
        let arguments = cx.argument::<JsArray>(1)?.to_vec(&mut cx)?;

        let coder = cx.this().downcast_or_throw::<JsBox<Coder>, _>(&mut cx)?;

        let kinds = coder.argument_types(&function).or_throw(&mut cx)?;

        // Cast JsValues into correct token types
        let tokens = kinds
            .iter()
            .zip(arguments.iter())
            .map(|(kind, value)| tokenize(&mut cx, kind, value))
            .collect::<Result<Vec<_>, _>>()
            .or_throw(&mut cx)?;

        let encoded_input = coder.encode_input(&function, &tokens).unwrap();
        Ok(cx.string(encoded_input))
    }

    fn js_decode_input(mut cx: FunctionContext) -> JsResult<JsArray> {
        let function = cx.argument::<JsString>(0)?.value(&mut cx);
        let data = cx.argument::<JsString>(1)?.value(&mut cx);

        let coder = cx.this().downcast_or_throw::<JsBox<Coder>, _>(&mut cx)?;

        // Decode calldata to tokens
        let tokens = coder.decode_input(&function, &data).or_throw(&mut cx)?;

        tokens_to_js(&mut cx, &tokens)
    }

    fn js_decode_output(mut cx: FunctionContext) -> JsResult<JsArray> {
        let function = cx.argument::<JsString>(0)?.value(&mut cx);
        let data = cx.argument::<JsString>(1)?.value(&mut cx);

        let coder = cx.this().downcast_or_throw::<JsBox<Coder>, _>(&mut cx)?;

        let tokens = coder.decode_output(&function, &data).or_throw(&mut cx)?;

        tokens_to_js(&mut cx, &tokens)
    }
}

fn tokens_to_js<'a, C: Context<'a>>(cx: &mut C, tokens: &[Token]) -> JsResult<'a, JsArray> {
    // See https://neon-bindings.com/docs/arrays#converting-a-rust-vector-to-an-array
    let result = JsArray::new(cx, tokens.len() as u32);
    for (i, token) in tokens.iter().enumerate() {
        let value = tokenize_out(token, cx)?;
        result.set(cx, i as u32, value)?;
    }
    Ok(result)
}

fn tokenize_out<'cx, C: Context<'cx>>(token: &Token, cx: &mut C) -> JsResult<'cx, JsValue> {
    Ok(match token {
        Token::Bool(b) => cx.boolean(*b).upcast(),
        Token::String(ref s) => cx.string(s.to_string()).upcast(),
        Token::Address(ref s) => cx.string(format!("0x{}", hex::encode(&s))).upcast(),
        Token::Bytes(ref bytes) | Token::FixedBytes(ref bytes) => {
            cx.string(format!("0x{}", hex::encode(bytes))).upcast()
        }
        Token::Uint(ref i) | Token::Int(ref i) => cx.string(i.to_string()).upcast(),
        // Arrays and Tuples will contain one of the above, or more arrays or tuples
        Token::Array(ref arr) | Token::FixedArray(ref arr) | Token::Tuple(ref arr) => {
            let value_array = JsArray::new(cx, arr.len() as u32);
            for (i, value) in arr.iter().enumerate() {
                let result = tokenize_out(value, cx)?;
                value_array.set(cx, i as u32, result)?;
            }
            value_array.upcast()
        }
    })
}

fn remove_hex_prefix(data_hex: &str) -> &str {
    // Remove any 0x prefix
    match &data_hex[..2] {
        "0x" => &data_hex[2..],
        _ => &data_hex,
    }
}

fn remove_bytes4(data_hex: &str) -> &str {
    // Remove any 0x prefix
    let s = remove_hex_prefix(&data_hex);
    &s[8..]
}

fn tokenize_address<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
) -> Result<[u8; 20], Throw> {
    let arg = value.downcast_or_throw::<JsString, _>(cx)?.value(cx);
    LenientTokenizer::tokenize_address(remove_hex_prefix(&arg)).or_throw(cx)
}

fn tokenize_string<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
) -> Result<String, Throw> {
    let arg = value.downcast_or_throw::<JsString, _>(cx)?.value(cx);
    LenientTokenizer::tokenize_string(&arg).or_throw(cx)
}

fn tokenize_bool<'cx, C: Context<'cx>>(cx: &mut C, value: &Handle<JsValue>) -> Result<bool, Throw> {
    let arg = value.downcast_or_throw::<JsBoolean, _>(cx)?.value(cx);
    Ok(arg)
}

fn tokenize_bytes<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
) -> Result<Vec<u8>, Throw> {
    let arg = value.downcast_or_throw::<JsString, _>(cx)?.value(cx);
    LenientTokenizer::tokenize_bytes(remove_hex_prefix(&arg)).or_throw(cx)
}

fn tokenize_fixed_bytes<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
    len: usize,
) -> Result<Vec<u8>, Throw> {
    let arg = value.downcast_or_throw::<JsString, _>(cx)?.value(cx);
    LenientTokenizer::tokenize_fixed_bytes(remove_hex_prefix(&arg), len).or_throw(cx)
}

fn tokenize_uint<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
) -> Result<[u8; 32], Throw> {
    let str = if value.is_a::<JsNumber, _>(cx) {
        let arg = value.downcast_or_throw::<JsNumber, _>(cx)?.value(cx);
        arg.to_string()
    } else {
        value.downcast_or_throw::<JsString, _>(cx)?.value(cx)
    };
    LenientTokenizer::tokenize_uint(&str).or_throw(cx)
}

fn tokenize_int<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
) -> Result<[u8; 32], Throw> {
    let str = if value.is_a::<JsNumber, _>(cx) {
        let arg = value.downcast_or_throw::<JsNumber, _>(cx)?.value(cx);
        arg.to_string()
    } else {
        value.downcast_or_throw::<JsString, _>(cx)?.value(cx)
    };
    LenientTokenizer::tokenize_int(&str).or_throw(cx)
}

fn tokenize_array<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
    param: &ParamType,
) -> Result<Vec<Token>, Throw> {
    let arr = value
        .downcast_or_throw::<JsArray, _>(cx)?
        .to_vec(cx)
        .or_throw(cx)?;
    let mut result = vec![];
    for (_i, v) in arr.iter().enumerate() {
        let token = tokenize(cx, param, v)?;
        result.push(token)
    }
    Ok(result)
}

fn tokenize_struct<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
    param: &[ParamType],
) -> Result<Vec<Token>, Throw> {
    let mut params = param.iter();
    let mut result = vec![];
    // If it's an array we assume it is in the correct order
    if value.is_a::<JsArray, _>(cx) {
        let arr = value
            .downcast_or_throw::<JsArray, _>(cx)?
            .to_vec(cx)
            .or_throw(cx)?;
        for (_i, v) in arr.iter().enumerate() {
            let p = params.next().ok_or(Error::InvalidData).or_throw(cx)?;
            let token = tokenize(cx, p, v)?;
            result.push(token)
        }
    } else {
        panic!("Unsupported object structure, use an array of ordered values");
    }
    Ok(result)
}

fn tokenize<'cx, C: Context<'cx>>(
    cx: &mut C,
    param: &ParamType,
    value: &Handle<JsValue>,
) -> Result<Token, Throw> {
    match *param {
        ParamType::Address => tokenize_address(cx, value).map(|a| Token::Address(a.into())),
        ParamType::String => tokenize_string(cx, value).map(Token::String),
        ParamType::Bool => tokenize_bool(cx, value).map(Token::Bool),
        ParamType::Bytes => tokenize_bytes(cx, value).map(Token::Bytes),
        ParamType::FixedBytes(len) => tokenize_fixed_bytes(cx, value, len).map(Token::FixedBytes),
        ParamType::Uint(_) => tokenize_uint(cx, value).map(Into::into).map(Token::Uint),
        ParamType::Int(_) => tokenize_int(cx, value).map(Into::into).map(Token::Int),
        ParamType::Array(ref p) => tokenize_array(cx, value, p).map(Token::Array),
        ParamType::FixedArray(ref p, _len) => tokenize_array(cx, value, p).map(Token::FixedArray),
        ParamType::Tuple(ref p) => tokenize_struct(cx, value, p).map(Token::Tuple),
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("coderNew", Coder::js_new)?;
    cx.export_function("coderEncodeInput", Coder::js_encode_input)?;
    cx.export_function("coderDecodeInput", Coder::js_decode_input)?;
    cx.export_function("coderDecodeOutput", Coder::js_decode_output)?;

    Ok(())
}
