mod result_ext;

use result_ext::ResultExt;
use neon::prelude::*;
use ethabi::{
    token::{LenientTokenizer, Token, Tokenizer},
    Contract, Error, Function, ParamType,
};
use std::{collections::HashMap, sync::Mutex};
use once_cell::sync::OnceCell;
use neon::object::This;

pub struct Coder(Contract);

impl Coder {
    pub fn new(abi_json: &str) -> anyhow::Result<Self> {
        Ok(Contract::load(abi_json.as_bytes()).map(Self)?)
    }

    pub fn argument_types(&self, function: &str) -> anyhow::Result<Vec<ParamType>> {
        Ok(self.0.function(function)?.inputs.iter().map(|param| param.kind.clone()).collect())
    }

    pub fn encode_input(&self, function: &str, arguments: &[Token]) -> anyhow::Result<String> {
        Ok(self.0.function(function)?.encode_input(arguments).map(hex::encode)?)
    }

    pub fn decode_input(&self, function: &str, data: &str) -> anyhow::Result<Vec<Token>> {
        let data = hex::decode(remove_hex_prefix(data))?;
        Ok(self.0.function(function)?.decode_input(&data)?)
    }

    pub fn decode_output(&self, function: &str, data: &str) -> anyhow::Result<Vec<Token>> {
        let data = hex::decode(remove_hex_prefix(data))?;
        Ok(self.0.function(function)?.decode_output(&data)?)
    }
}

declare_types! {
    pub class JsCoder for Coder {
        init(mut cx) {
            Coder::new(cx.argument::<JsString>(0)?.value().as_ref())
            .or_else(|e| cx.throw_error(format!("{}", e)))
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
            .map(|(kind, value)| tokenize(kind, value, &mut cx))
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

fn tokens_to_js<'cx, T: This>(
    cx: &mut CallContext<'cx, T>,
    tokens: &[Token],
) -> JsResult<'cx, JsValue>
{
    let result = JsArray::new(cx, tokens.len() as u32);
    for (i, token) in tokens.iter().enumerate() {
        let value = tokenize_out(token, cx)
        .or_throw(cx)?;
        result.set(cx, i as u32, value)?;
    }
    Ok(result.upcast())
}

static INSTANCE: OnceCell<Mutex<HashMap<String, Contract>>> = OnceCell::new();

fn load_abi(mut cx: FunctionContext) -> JsResult<JsBoolean> {
    let id_h: Handle<JsString> = cx.argument(0)?;
    let id = id_h.downcast::<JsString>().unwrap().value();

    let abi_json_h: Handle<JsString> = cx.argument(1)?;
    let abi_json = abi_json_h.downcast::<JsString>().unwrap().value();

    INSTANCE
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .insert(id, Contract::load(abi_json.as_bytes()).unwrap());
    Ok(cx.boolean(true))
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

fn tokenize_address(value: &Handle<JsValue>) -> Result<[u8; 20], Error> {
    let arg = value.downcast::<JsString>().unwrap().value();
    LenientTokenizer::tokenize_address(remove_hex_prefix(&arg))
}

fn tokenize_string(value: &Handle<JsValue>) -> Result<String, Error> {
    let arg = value.downcast::<JsString>().unwrap().value();
    LenientTokenizer::tokenize_string(&arg)
}

fn tokenize_bool(value: &Handle<JsValue>) -> Result<bool, Error> {
    let arg = value.downcast::<JsBoolean>().unwrap().value();
    Ok(arg)
}

fn tokenize_bytes(value: &Handle<JsValue>) -> Result<Vec<u8>, Error> {
    let arg = value.downcast::<JsString>().unwrap().value();
    LenientTokenizer::tokenize_bytes(remove_hex_prefix(&arg))
}

fn tokenize_fixed_bytes(value: &Handle<JsValue>, len: usize) -> Result<Vec<u8>, Error> {
    let arg = value.downcast::<JsString>().unwrap().value();
    LenientTokenizer::tokenize_fixed_bytes(remove_hex_prefix(&arg), len)
}

fn tokenize_uint(value: &Handle<JsValue>) -> Result<[u8; 32], Error> {
    let str = if value.is_a::<JsNumber>() {
        let arg = value.downcast::<JsNumber>().unwrap().value();
        arg.to_string()
    } else {
        value.downcast::<JsString>().unwrap().value()
    };
    LenientTokenizer::tokenize_uint(&str)
}

fn tokenize_int(value: &Handle<JsValue>) -> Result<[u8; 32], Error> {
    let str = if value.is_a::<JsNumber>() {
        let arg = value.downcast::<JsNumber>().unwrap().value();
        arg.to_string()
    } else {
        value.downcast::<JsString>().unwrap().value()
    };
    LenientTokenizer::tokenize_int(&str)
}

fn tokenize_array<T: neon::object::This>(
    value: &Handle<JsValue>,
    param: &ParamType,
    cx: &mut CallContext<'_, T>,
) -> Result<Vec<Token>, Error> {
    let arr = value.downcast::<JsArray>().unwrap().to_vec(cx).unwrap();
    let mut result = vec![];
    for (_i, v) in arr.iter().enumerate() {
        let token = tokenize(param, v, cx)?;
        result.push(token)
    }
    Ok(result)
}

fn tokenize_struct<T: neon::object::This>(
    value: &Handle<JsValue>,
    param: &[ParamType],
    cx: &mut CallContext<'_, T>,
) -> Result<Vec<Token>, Error> {
    let mut params = param.iter();
    let mut result = vec![];
    // If it's an array we assume it is in the correct order
    if value.is_a::<JsArray>() {
        let arr = value.downcast::<JsArray>().unwrap().to_vec(cx).unwrap();
        for (_i, v) in arr.iter().enumerate() {
            let p = params.next().ok_or(Error::InvalidData)?;
            let token = tokenize(p, v, cx)?;
            result.push(token)
        }
    } else {
        panic!("Unsupported object structure, use an array of ordered values");
    }
    Ok(result)
}

fn tokenize<T: neon::object::This>(
    param: &ParamType,
    value: &Handle<JsValue>,
    cx: &mut CallContext<'_, T>,
) -> Result<Token, Error> {
    match *param {
        ParamType::Address => tokenize_address(value).map(|a| Token::Address(a.into())),
        ParamType::String => tokenize_string(value).map(Token::String),
        ParamType::Bool => tokenize_bool(value).map(Token::Bool),
        ParamType::Bytes => tokenize_bytes(value).map(Token::Bytes),
        ParamType::FixedBytes(len) => tokenize_fixed_bytes(value, len).map(Token::FixedBytes),
        ParamType::Uint(_) => tokenize_uint(value).map(Into::into).map(Token::Uint),
        ParamType::Int(_) => tokenize_int(value).map(Into::into).map(Token::Int),
        ParamType::Array(ref p) => tokenize_array(value, p, cx).map(Token::Array),
        ParamType::FixedArray(ref p, _len) => tokenize_array(value, p, cx).map(Token::FixedArray),
        ParamType::Tuple(ref p) => tokenize_struct(value, p, cx).map(Token::Tuple),
    }
}

fn tokenize_out<'cx, T: neon::object::This>(
    token: &ethabi::Token,
    cx: &mut CallContext<'cx, T>,
) -> Result<Handle<'cx, JsValue>, Error> {
    let value: Handle<JsValue> = match token {
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
                value_array.set(cx, i as u32, result).unwrap();
            }
            value_array.upcast()
        }
    };
    Ok(value)
}

fn parse_tokens(
    params: &[(ParamType, &Handle<JsValue>)],
    cx: &mut FunctionContext,
) -> anyhow::Result<Vec<Token>> {
    params
        .iter()
        .map(|&(ref param, value)| tokenize(param, value, cx))
        .collect::<Result<_, _>>()
        .map_err(From::from)
}

fn encode_input(mut cx: FunctionContext) -> JsResult<JsString> {
    // ID (0)
    // function name (1)
    // args array (2)
    let id_h: Handle<JsString> = cx.argument(0)?;
    let id = id_h.downcast::<JsString>().unwrap().value();

    let function_signature_h: Handle<JsString> = cx.argument(1)?;
    let args_h: Handle<JsArray> = cx.argument(2)?;

    let function_signature = function_signature_h.downcast::<JsString>().unwrap().value();
    let args_vec: Vec<Handle<JsValue>> = args_h.to_vec(&mut cx)?;

    let function: Function = INSTANCE
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .get(&id)
        .unwrap()
        .functions_by_name(&function_signature)
        .unwrap()[0]
        .clone();

    let params: Vec<_> = function
        .inputs
        .iter()
        .map(|param| param.kind.clone())
        .zip(args_vec.iter().map(|v| v as &Handle<JsValue>))
        .collect();
    let tokens = parse_tokens(&params, &mut cx).unwrap();
    let encoded = function.encode_input(&tokens).unwrap();
    Ok(cx.string(hex::encode(&encoded)))
}

fn decode_output(mut cx: FunctionContext) -> JsResult<JsArray> {
    let id_h: Handle<JsString> = cx.argument(0)?;
    let function_signature_h: Handle<JsString> = cx.argument(1)?;
    let data_h: Handle<JsString> = cx.argument(2)?;

    let id = id_h.downcast::<JsString>().unwrap().value();
    let function_signature = function_signature_h.downcast::<JsString>().unwrap().value();
    let data_hex = data_h.downcast::<JsString>().unwrap().value();

    let function: Function = INSTANCE
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .get(&id)
        .unwrap()
        .functions_by_name(&function_signature)
        .unwrap()[0]
        .clone();

    let data: Vec<u8> = hex::decode(remove_hex_prefix(&data_hex)).unwrap();
    let tokens = function.decode_output(&data).unwrap();

    let result_array = JsArray::new(&mut cx, tokens.len() as u32);

    for (i, token) in tokens.iter().enumerate() {
        let result = tokenize_out(token, &mut cx).unwrap();
        result_array.set(&mut cx, i as u32, result)?;
    }

    Ok(result_array)
}

fn decode_input(mut cx: FunctionContext) -> JsResult<JsArray> {
    let id_h: Handle<JsString> = cx.argument(0)?;
    let function_signature_h: Handle<JsString> = cx.argument(1)?;
    let data_h: Handle<JsString> = cx.argument(2)?;

    let id = id_h.downcast::<JsString>().unwrap().value();
    let function_signature = function_signature_h.downcast::<JsString>().unwrap().value();
    let data_hex = data_h.downcast::<JsString>().unwrap().value();

    let function: Function = INSTANCE
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .get(&id)
        .unwrap()
        .functions_by_name(&function_signature)
        .unwrap()[0]
        .clone();

    let data: Vec<u8> = hex::decode(&remove_bytes4(&data_hex)).unwrap();
    let tokens = function.decode_input(&data).unwrap();

    let result_array = JsArray::new(&mut cx, tokens.len() as u32);

    for (i, token) in tokens.iter().enumerate() {
        let result = tokenize_out(token, &mut cx).unwrap();
        result_array.set(&mut cx, i as u32, result)?;
    }

    Ok(result_array)
}

fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string("hello world"))
}

register_module!(mut cx, {
    INSTANCE.set(Mutex::new(HashMap::new())).unwrap();

    cx.export_class::<JsCoder>("Coder")?;
    cx.export_function("hello", hello)?;
    cx.export_function("loadAbi", load_abi)?;
    cx.export_function("encodeInput", encode_input)?;
    cx.export_function("decodeInput", decode_input)?;
    cx.export_function("decodeOutput", decode_output)?;
    Ok(())
});
