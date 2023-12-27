use std::collections::HashMap;

use heck::{ToPascalCase, ToSnekCase};
use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use quote::{quote, ToTokens};
use serde::Deserialize;
use serde_json::{Map, Value};
use syn::parse::Parse;
use syn::{parse_quote, ItemEnum, ItemImpl, ItemMod, ItemStruct, Variant};

#[derive(Parser)]
#[grammar = "ts.pest"]
struct TsParser;

#[derive(Debug, Deserialize)]
pub struct SubsystemCommand {
    #[serde(rename = "ID")]
    pub id: u8,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: CommandType,
    pub request: Vec<Parameter>,
    #[serde(default)]
    pub response: Option<Vec<Parameter>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub name: String,
    pub parameter_type: ParameterType,
}

#[derive(Debug, Deserialize)]
pub enum ParameterType {
    #[serde(rename = "ParameterType.BUFFER")]
    Buffer,
    #[serde(rename = "ParameterType.BUFFER8")]
    Buffer8,
    #[serde(rename = "ParameterType.BUFFER16")]
    Buffer16,
    #[serde(rename = "ParameterType.BUFFER18")]
    Buffer18,
    #[serde(rename = "ParameterType.BUFFER32")]
    Buffer32,
    #[serde(rename = "ParameterType.BUFFER42")]
    Buffer42,
    #[serde(rename = "ParameterType.BUFFER100")]
    Buffer100,
    #[serde(rename = "ParameterType.IEEEADDR")]
    IeeeAddr,
    #[serde(rename = "ParameterType.LIST_ASSOC_DEV")]
    ListAssocDev,
    #[serde(rename = "ParameterType.LIST_BIND_TABLE")]
    ListBindTable,
    #[serde(rename = "ParameterType.LIST_NEIGHBOR_LQI")]
    ListNeighborLqi,
    #[serde(rename = "ParameterType.LIST_NETWORK")]
    ListNetwork,
    #[serde(rename = "ParameterType.LIST_ROUTING_TABLE")]
    ListRoutingTable,
    #[serde(rename = "ParameterType.LIST_UINT8")]
    ListUint8,
    #[serde(rename = "ParameterType.LIST_UINT16")]
    ListUint16,
    #[serde(rename = "ParameterType.UINT8")]
    Uint8,
    #[serde(rename = "ParameterType.UINT16")]
    Uint16,
    #[serde(rename = "ParameterType.UINT32")]
    Uint32,
    #[serde(rename = "ParameterType.INT8")]
    Int8,
}

impl ParameterType {
    fn ty(&self) -> syn::Type {
        match self {
            ParameterType::Buffer => parse_quote! { Buffer },
            ParameterType::Buffer8 => parse_quote! { BufferN<8> },
            ParameterType::Buffer16 => parse_quote! { BufferN<16> },
            ParameterType::Buffer18 => parse_quote! { BufferN<18> },
            ParameterType::Buffer32 => parse_quote! { BufferN<32> },
            ParameterType::Buffer42 => parse_quote! { BufferN<42> },
            ParameterType::Buffer100 => parse_quote! { BufferN<100> },
            ParameterType::IeeeAddr => parse_quote! { IeeeAddr },
            ParameterType::ListAssocDev => parse_quote! { Vec<AssocDev> },
            ParameterType::ListBindTable => parse_quote! { Vec<BindTable> },
            ParameterType::ListNeighborLqi => parse_quote! { Vec<NeighborLqi> },
            ParameterType::ListNetwork => parse_quote! { Vec<Network> },
            ParameterType::ListRoutingTable => parse_quote! { Vec<RoutingTable> },
            ParameterType::ListUint8 => parse_quote! { Vec<u8> },
            ParameterType::ListUint16 => parse_quote! { Vec<u16> },
            ParameterType::Uint8 => parse_quote! { u8 },
            ParameterType::Uint16 => parse_quote! { u16 },
            ParameterType::Uint32 => parse_quote! { u32 },
            ParameterType::Int8 => parse_quote! { i8 },
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum CommandType {
    #[serde(rename = "CommandType.SREQ")]
    SyncRequest,
    #[serde(rename = "CommandType.AREQ")]
    AsyncRequest,
}

#[allow(clippy::result_large_err)]
fn parse_source_file(file: &str) -> Result<Value, Error<Rule>> {
    let source = TsParser::parse(Rule::table, file)?.next().unwrap();

    fn parse_value(pair: Pair<Rule>) -> Value {
        match pair.as_rule() {
            Rule::object => Value::Object(
                pair.into_inner()
                    .map(|pair| {
                        let mut inner_rules = pair.into_inner();
                        let name = parse_value(inner_rules.next().unwrap())
                            .as_str()
                            .unwrap()
                            .to_string();
                        let value = parse_value(inner_rules.next().unwrap());
                        (name, value)
                    })
                    .collect(),
            ),
            Rule::array => Value::Array(pair.into_inner().map(parse_value).collect()),
            Rule::string => Value::String(pair.into_inner().next().unwrap().as_str().to_string()),
            Rule::number => Value::Number(pair.as_str().parse().unwrap()),
            Rule::boolean => Value::Bool(pair.as_str().parse().unwrap()),
            Rule::null => Value::Null,
            Rule::path => Value::String(pair.as_str().to_string()),
            Rule::ident => Value::String(pair.as_str().to_string()),
            Rule::COMMENT
            | Rule::EOI
            | Rule::WHITESPACE
            | Rule::char
            | Rule::inner
            | Rule::table
            | Rule::key
            | Rule::pair
            | Rule::value => unreachable!("{:?}", pair.as_str()),
        }
    }

    Ok(parse_value(source))
}

fn main() {
    let unparsed_file = include_str!("./source.ts");
    match parse_source_file(unparsed_file) {
        Ok(json) => {
            let Value::Object(map) = json else {
                panic!("Expected object");
            };

            let subsystems = map.into_iter().map(|(k, v)| {
                let commands: Vec<SubsystemCommand> = serde_json::from_value(v).unwrap();
                (k, commands)
            });

            let mut root_enum: ItemEnum = parse_quote! {
                pub enum Subsystem {}
            };

            let mut subsystem_from_impls = Vec::<ItemImpl>::new();

            for (name, commands) in subsystems {
                let subsystem_name = name.split('.').last().unwrap().to_pascal_case();
                let variant_name = quote::format_ident!("{}", subsystem_name);
                let enum_name =
                    quote::format_ident!("{}Subsystem", subsystem_name.to_pascal_case());

                root_enum
                    .variants
                    .push(parse_quote! { #variant_name(#enum_name) });

                let mut subsystem_enum: ItemEnum = parse_quote! {
                    pub enum #enum_name {}
                };

                let mut command_structs = Vec::<ItemStruct>::new();
                let mut command_from_impls = Vec::<ItemImpl>::new();

                for command in commands {
                    let variant_name = quote::format_ident!("{}", command.name.to_pascal_case());
                    let struct_name = quote::format_ident!("{}", command.name.to_pascal_case());

                    subsystem_enum
                        .variants
                        .push(parse_quote! { #variant_name(#struct_name) });

                    let mut field_names = Vec::new();
                    let mut field_types = Vec::new();

                    for param in command.request {
                        let mut name = param.name.to_snek_case();

                        if name == "type" {
                            name = "r#type".to_string();
                        }

                        let ty = param.parameter_type.ty();

                        field_names.push(quote::format_ident!("{}", name));
                        field_types.push(ty);
                    }

                    command_structs.push(parse_quote! {
                        pub struct #struct_name {
                            #( pub #field_names: #field_types ),*
                        }
                    });

                    if let Some(response) = command.response {
                        let struct_name =
                            quote::format_ident!("{}Response", command.name.to_pascal_case());

                        let mut field_names = Vec::new();
                        let mut field_types = Vec::new();

                        for param in response {
                            let mut name = param.name.to_snek_case();

                            if name == "type" {
                                name = "r#type".to_string();
                            }

                            let ty = param.parameter_type.ty();

                            field_names.push(quote::format_ident!("{}", name));
                            field_types.push(ty);
                        }

                        command_structs.push(parse_quote! {
                            pub struct #struct_name {
                                #( pub #field_names: #field_types ),*
                            }
                        });
                    }

                    command_from_impls.push(parse_quote! {
                        impl From<#struct_name> for Command {
                            fn from(command: #struct_name) -> Self {
                                Command::#variant_name(command)
                            }
                        }
                    });
                }

                let subsystem_mod = quote::format_ident!("{}", subsystem_name.to_snek_case());

                let subsystem_module: ItemMod = parse_quote! {
                    pub mod #subsystem_mod {
                        #( #command_structs )*
                        #( #command_from_impls )*
                    }
                };

                subsystem_from_impls.push(parse_quote! {
                    impl From<#enum_name> for Subsystem {
                        fn from(subsystem: #enum_name) -> Self {
                            Subsystem::#variant_name(subsystem)
                        }
                    }
                });

                println!("{}", subsystem_enum.into_token_stream());

                println!("{}", subsystem_module.into_token_stream());
            }

            println!("{}", quote! { #root_enum });

            println!("{}", quote! { #(#subsystem_from_impls)* });
        }
        Err(e) => {
            println!("{}", e);
        }
    }
}
