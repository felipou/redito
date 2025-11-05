use clap::{Arg, ArgAction, ArgMatches, Command, command, value_parser};
use schemars::{SchemaGenerator, generate::SchemaSettings};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};

use crate::settings::AppConfig;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum CommandFixedType {
    String,
}

#[derive(Debug, Deserialize)]
struct CommandType {
    #[expect(unused)]
    #[serde(rename = "type")]
    ctype: CommandFixedType,

    #[serde(rename = "const")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct CommandProps {
    #[serde(flatten)]
    props: serde_json::Map<String, Value>,
    command_type: CommandType,
}

#[derive(Debug, Deserialize)]
struct CommandDef {
    properties: CommandProps,
    required: Option<Vec<String>>,
}

fn add_args_from_root_schema(cmd: Command, schema: Value, defaults: &Value) -> Command {
    let mut cmd = cmd;

    let Value::Object(mut props) = schema else {
        panic!("Invalid root schema is not of type object");
    };

    if let Some(command) = props.remove("command") {
        let Value::Object(mut command_obj) = command else {
            panic!("Invalid command is not object: {command}");
        };

        let Some(command_options) = command_obj.remove("oneOf") else {
            panic!("Invalid command does not have oneOf attribute: {command_obj:?}");
        };

        let Value::Array(command_options) = command_options else {
            panic!("Invalid command options is not array: {command_options}");
        };

        for command_def in command_options {
            let command_def: CommandDef =
                serde_json::from_value(command_def).expect("invalid command definition");

            let command_type = command_def.properties.command_type.name.replace("_", "-");

            let subcmd = add_args_from_schema(
                Command::new(&command_type).about(&command_type),
                command_def.properties.props,
                &defaults.get("command").cloned().unwrap_or(Value::Null),
                None,
                Some("command"),
                &command_def.required.unwrap_or_default(),
                false,
            );
            cmd = cmd.subcommand(subcmd);
        }
    }

    add_args_from_schema(cmd, props, defaults, None, None, &[], true)
}

fn deserialize_arg_type<'de, D>(deserializer: D) -> Result<ArgType, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum UnionType {
        Str(ArgType),
        Vec(Vec<Value>),
    }

    match UnionType::deserialize(deserializer)? {
        UnionType::Str(arg_type) => Ok(arg_type),

        UnionType::Vec(v) => {
            if v.len() != 2 || v[1] != "null" {
                Err(serde::de::Error::custom(
                    "arg types only support the second type null",
                ))
            } else {
                Ok(serde_json::from_value(v[0].clone()).map_err(serde::de::Error::custom)?)
            }
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ArgType {
    String,
    Integer,
    Boolean,
    Object,
    Null,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ArgFormat {
    Uint,
    Uint8,
    Uint16,
}

#[derive(Debug, Deserialize)]
struct ArgDef {
    #[serde(rename = "type", deserialize_with = "deserialize_arg_type")]
    arg_type: ArgType,
    properties: Option<serde_json::Map<String, Value>>,
    required: Option<Vec<String>>,
    format: Option<ArgFormat>,
}

fn add_args_from_schema(
    cmd: Command,
    props: serde_json::map::Map<String, Value>,
    defaults: &Value,
    flag_prefix: Option<&str>,
    id_prefix: Option<&str>,
    required: &[String],
    global: bool,
) -> Command {
    let mut cmd = cmd;

    for (key, subschema) in props {
        let flag_name = if let Some(prefix) = flag_prefix {
            format!("{}-{}", prefix, key.replace('_', "-"))
        } else {
            key.replace('_', "-")
        };

        let arg_id = if let Some(prefix) = id_prefix {
            format!("{}__{}", prefix, key)
        } else {
            key.clone()
        };

        let default_val = defaults.get(&key).cloned().unwrap_or(Value::Null);

        let arg_def: ArgDef =
            serde_json::from_value(subschema).expect("Invalid argument definition");

        if arg_def.arg_type == ArgType::Object {
            cmd = add_args_from_schema(
                cmd,
                arg_def
                    .properties
                    .expect("properties of type object should exist"),
                &default_val,
                Some(&flag_name),
                Some(&arg_id),
                arg_def.required.as_deref().unwrap_or(&[]),
                global,
            );
        } else {
            let arg = setup_arg(
                arg_def,
                required.contains(&key),
                global,
                flag_name,
                arg_id,
                default_val,
            );

            cmd = cmd.arg(arg);
        }
    }

    cmd
}

fn setup_arg(
    arg_def: ArgDef,
    required: bool,
    global: bool,
    flag_name: String,
    arg_id: String,
    default_val: Value,
) -> Arg {
    let mut arg = Arg::new(arg_id)
        .long(&flag_name)
        .value_name(flag_name.to_uppercase().replace("-", "_"))
        .global(global);

    if global {
        arg = arg.display_order(0);
        arg = arg.help_heading("GLOBAL");
    } else {
        arg = arg.display_order(1);
    }

    if required && default_val.is_null() && arg_def.arg_type != ArgType::Boolean {
        arg = arg.required(true);
    }

    match arg_def.arg_type {
        ArgType::Integer => {
            match arg_def.format {
                Some(ArgFormat::Uint) => {
                    arg = arg.value_parser(value_parser!(usize));
                }
                Some(ArgFormat::Uint8) => {
                    arg = arg.value_parser(value_parser!(u8));
                }
                Some(ArgFormat::Uint16) => {
                    arg = arg.value_parser(value_parser!(u16));
                }
                None => {
                    arg = arg.value_parser(value_parser!(i32));
                }
            }

            if !default_val.is_null() {
                arg = arg.default_value(default_val.to_string());
            }
        }
        ArgType::String => {
            if !default_val.is_null() {
                arg = arg.default_value(default_val.as_str().unwrap().to_string());
            }
        }
        ArgType::Boolean => {
            arg = arg.action(ArgAction::SetTrue);
            if default_val == Value::Bool(true) {
                arg = arg.default_value("true");
            }
        }
        _ => {
            panic!("invalid value_type: {arg_def:?}")
        }
    }
    arg
}

fn insert_composite_value(key: &str, value: Value, cli_json: &mut Value) {
    let tokens_split: Vec<&str> = key.split("__").collect();
    let mut cursor = cli_json.as_object_mut().unwrap();
    for part in &tokens_split[..tokens_split.len() - 1] {
        cursor = cursor
            .entry(part.to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .unwrap();
    }

    cursor.insert(tokens_split.last().unwrap().to_string(), json!(value));
}

fn add_arg_matches_to_json(matches: &ArgMatches, cli_json: &mut Value) {
    for id in matches.ids() {
        let value = if let Ok(Some(str_value)) = matches.try_get_one::<String>(id.as_str()) {
            json!(str_value)
        } else if let Ok(Some(bool_value)) = matches.try_get_one::<bool>(id.as_str()) {
            json!(bool_value)
        } else if let Ok(Some(int_value)) = matches.try_get_one::<usize>(id.as_str()) {
            json!(int_value)
        } else if let Ok(Some(int_value)) = matches.try_get_one::<u8>(id.as_str()) {
            json!(int_value)
        } else if let Ok(Some(int_value)) = matches.try_get_one::<u16>(id.as_str()) {
            json!(int_value)
        } else {
            panic!("Unknown arg value type: {id}");
        };

        insert_composite_value(id.as_str(), value, cli_json);
    }
}

pub fn setup_from_schema(base_config: serde_json::Value) -> anyhow::Result<Value> {
    let mut schema_settings = SchemaSettings::default();
    schema_settings.inline_subschemas = true;
    let mut schema = SchemaGenerator::new(schema_settings)
        .into_root_schema_for::<AppConfig>()
        .to_value();

    let schema = schema
        .as_object_mut()
        .expect("schema is not an object")
        .remove("properties")
        .expect("missing properties attribute");

    let mut cmd = command!();
    cmd = add_args_from_root_schema(cmd, schema, &base_config);

    let matches = cmd.get_matches();

    let mut cli_json = json!({});

    add_arg_matches_to_json(&matches, &mut cli_json);

    if let Some((command_name, submatches)) = matches.subcommand() {
        add_arg_matches_to_json(submatches, &mut cli_json);
        insert_composite_value(
            "command__command_type",
            json!(command_name.replace("-", "_")),
            &mut cli_json,
        );
    }

    Ok(cli_json)
}
