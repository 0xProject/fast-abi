mod result_ext;

use ethabi::{
    token::{LenientTokenizer, Token, Tokenizer},
    Contract, Error, ParamType,
};
use neon::{prelude::*, result::Throw};
use result_ext::ResultExt;

pub struct Coder(Contract);

impl Coder {
    pub fn new(abi_json: &str) -> anyhow::Result<Self> {
        Ok(Contract::load(abi_json.as_bytes()).map(Self)?)
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

declare_types! {
    pub class JsCoder for Coder {
        init(mut cx) {
            Coder::new(cx.argument::<JsString>(0)?.value().as_ref())
            .or_throw(&mut cx)
        }

        method encodeInput(mut cx) {
            let this = cx.this();
            let function = cx.argument::<JsString>(0)?.value();
            let arguments = cx.argument::<JsArray>(1)?.to_vec(&mut cx)?;

            // Fetch argument types
            let kinds = cx.borrow(&this, |coder| coder.argument_types(&function))
            .or_throw(&mut cx)?;

            // Cast JsValues into correct token types
            let tokens = kinds.iter().zip(arguments.iter())
            .map(|(kind, value)| tokenize(&mut cx, kind, value))
            .collect::<Result<Vec<_>,_>>()
            .or_throw(&mut cx)?;

            // Encode tokenized arguments
            cx.borrow(&this, |coder| coder.encode_input(&function, &tokens))
            .or_throw(&mut cx)
            .map(|s| cx.string(s).upcast())
        }

        method decodeInput(mut cx) {
            let this = cx.this();
            let function = cx.argument::<JsString>(0)?.value();
            let data = cx.argument::<JsString>(1)?.value();

            // Decode calldata to tokens
            let tokens = cx.borrow(&this, |coder| coder.decode_input(&function, &data))
            .or_throw(&mut cx)?;
            tokens_to_js(&mut cx, &tokens)
        }

        method decodeOutput(mut cx) {
            let this = cx.this();
            let function = cx.argument::<JsString>(0)?.value();
            let data = cx.argument::<JsString>(1)?.value();

            // Decode calldata to tokens
            let tokens = cx.borrow(&this, |coder| coder.decode_output(&function, &data))
            .or_throw(&mut cx)?;
            tokens_to_js(&mut cx, &tokens)
        }
    }
}

fn tokens_to_js<'cx, C: Context<'cx>>(cx: &mut C, tokens: &[Token]) -> JsResult<'cx, JsValue> {
    let result = JsArray::new(cx, tokens.len() as u32);
    for (i, token) in tokens.iter().enumerate() {
        let value = tokenize_out(token, cx)?;
        result.set(cx, i as u32, value)?;
    }
    Ok(result.upcast())
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
    let arg = value.downcast_or_throw::<JsString, _>(cx)?.value();
    LenientTokenizer::tokenize_address(remove_hex_prefix(&arg)).or_throw(cx)
}

fn tokenize_string<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
) -> Result<String, Throw> {
    let arg = value.downcast_or_throw::<JsString, _>(cx)?.value();
    LenientTokenizer::tokenize_string(&arg).or_throw(cx)
}

fn tokenize_bool<'cx, C: Context<'cx>>(cx: &mut C, value: &Handle<JsValue>) -> Result<bool, Throw> {
    let arg = value.downcast_or_throw::<JsBoolean, _>(cx)?.value();
    Ok(arg)
}

fn tokenize_bytes<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
) -> Result<Vec<u8>, Throw> {
    let arg = value.downcast_or_throw::<JsString, _>(cx)?.value();
    LenientTokenizer::tokenize_bytes(remove_hex_prefix(&arg)).or_throw(cx)
}

fn tokenize_fixed_bytes<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
    len: usize,
) -> Result<Vec<u8>, Throw> {
    let arg = value.downcast_or_throw::<JsString, _>(cx)?.value();
    LenientTokenizer::tokenize_fixed_bytes(remove_hex_prefix(&arg), len).or_throw(cx)
}

fn tokenize_uint<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
) -> Result<[u8; 32], Throw> {
    let str = if value.is_a::<JsNumber>() {
        let arg = value.downcast_or_throw::<JsNumber, _>(cx)?.value();
        arg.to_string()
    } else {
        value.downcast_or_throw::<JsString,_>(cx)?.value()
    };
    LenientTokenizer::tokenize_uint(&str).or_throw(cx)
}

fn tokenize_int<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
) -> Result<[u8; 32], Throw> {
    let str = if value.is_a::<JsNumber>() {
        let arg = value.downcast_or_throw::<JsNumber, _>(cx)?.value();
        arg.to_string()
    } else {
        value.downcast_or_throw::<JsString, _>(cx)?.value()
    };
    LenientTokenizer::tokenize_int(&str).or_throw(cx)
}

fn tokenize_array<'cx, C: Context<'cx>>(
    cx: &mut C,
    value: &Handle<JsValue>,
    param: &ParamType,
) -> Result<Vec<Token>, Throw> {
    let arr = value.downcast_or_throw::<JsArray,_>(cx)?.to_vec(cx).or_throw(cx)?;
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
    if value.is_a::<JsArray>() {
        let arr = value.downcast_or_throw::<JsArray, _>(cx)?.to_vec(cx).or_throw(cx)?;
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

register_module!(mut cx, { cx.export_class::<JsCoder>("Coder") });
