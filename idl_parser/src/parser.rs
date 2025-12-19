mod annotations;
mod components;
mod core;
mod extended_data_types;
mod interfaces;
mod template;
mod value_types;

use crate::{expr::AnnotationAndDef, parser::core::skip_space_and_comment0};
use nom::{
    combinator::eof,
    error::{Error, ErrorKind},
    Finish, IResult,
};
use nom_greedyerror::{convert_error, GreedyError};
use nom_locate::LocatedSpan;

type Span<'a> = LocatedSpan<&'a str>;
type PResult<'a, OUT> = IResult<Span<'a>, OUT, GreedyError<Span<'a>, ErrorKind>>;
type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub fn parse(input: &str) -> Result<Vec<AnnotationAndDef>, DynError> {
    fn do_parse(input: &str) -> PResult<'_, Vec<AnnotationAndDef>> {
        let mut input = Span::new(input);

        let mut result = Vec::new();
        loop {
            let (i, m) = core::parse_definition(input)?;
            let (i, _) = skip_space_and_comment0(i)?;

            result.push(m);

            if let Ok((_, _)) = eof::<Span, Error<Span>>(i) {
                break;
            }

            input = i;
        }

        Ok((input, result))
    }

    match do_parse(input).finish() {
        Ok((_, definitions)) => Ok(definitions),
        Err(e) => {
            let msg = convert_error(input, e);
            Err(msg.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{core::skip_space_and_comment0, Span};
    use crate::parse;
    use nom_greedyerror::convert_error;

    #[test]
    fn parser() {
        let input = r#"
module autoware_auto_vehicle_msgs {
    module msg {
        @verbatim (language="comment", text=
        " Command to avoid stationary locking \(Park gear, Parking Brake, etc.\)."
        " This command assumes that the VehicleInterface or lower levels are controlling"
        " the mechanism by which a vehicle is prevented from moving \(\"stationary locking\"\)"
        " and will automatically attempt to apply stationary locking if a 0-velocity command"
        " is applied for an arbitrary period of time. Setting the avoid_stationary_locking"
        " field to TRUE tells the VehicleInterface or lower levels to avoid making this transition.")

        struct StationaryLockingCommand {
            builtin_interfaces::msg::Time stamp;

            @default (value=FALSE)
            boolean avoid_stationary_locking;
        };
    };
};"#;

        match parse(input) {
            Ok(definitions) => {
                println!("{:#?}", definitions);
            }
            Err(e) => {
                eprintln!("{e}");
                panic!();
            }
        }
    }

    #[test]
    fn comment() {
        let input = r#"/*
abc */

// abc
// def"#;

        match skip_space_and_comment0(Span::new(input)) {
            Ok((input, _)) => {
                println!("{input}");
            }
            Err(nom::Err::Error(e)) => {
                let msg = convert_error(input, e);
                eprintln!("{msg}");
                panic!();
            }
            Err(_) => (),
        }
    }

    #[test]
    fn parser_srv() {
        let input = r#"
// generated from rosidl_adapter/resource/srv.idl.em
// with input from example_msg/srv/AddThreeInts.srv
// generated code does not contain a copyright notice


module example_msg {
    module srv {
        struct AddThreeInts_Request {
            @default (value=0.0)
            int64 a;

            int64 b;

            int64 c;
        };
        module AddThreeInts_Response_Constants {
            const string PLANNING = "Planning";
        };
        struct AddThreeInts_Response {
            int64 sum;
        };
    };
};"#;

        match parse(input) {
            Ok(definitions) => {
                println!("{:?}", definitions);
            }
            Err(e) => {
                eprintln!("{e}");
                panic!();
            }
        }
    }

    #[test]
    fn parse_typedef() {
        let input = "typedef string<10> string__10__2[2];";
        match parse(input) {
            Ok(definitions) => {
                println!("{:?}", definitions);
            }
            Err(e) => {
                eprintln!("{e}");
                panic!();
            }
        }
    }

    #[test]
    fn parse_sequence() {
        let input = "module a { struct b { sequence<string<10>> dynamic_array_bounded_str; }; };";
        match parse(input) {
            Ok(definitions) => {
                println!("{:?}", definitions);
            }
            Err(e) => {
                eprintln!("{e}");
                panic!();
            }
        }
    }

    #[test]
    fn parser_complex() {
        let input = r#"
module example_msg {
    module msg {
        typedef int32 int32__10[10];
        module StdMsg_Constants {
            @verbatim (language="comment", text=
                "constant")
            const int32 XX = 20;
            const string INITIALIZING_VEHICLE = "InitializingVehicle a\"a";
            const string WAITING_FOR_ROUTE = "WaitingForRoute";
            const string PLANNING = "Planning";
        };

        @verbatim (language="comment", text="http://wiki.ros.org/msg")
        struct StdMsg {
            boolean a;

            int8 b;

            uint8 c;

            int16 d;

            uint16 e;

            int32 f;

            uint32 g;

            int64 h;

            uint64 i;

            float j;

            double k;

            string l;

            @verbatim (language="comment", text=
                "time m" "\n"
                "duration n" "\n"
                "array")
            sequence<int32> o;

            int32__10 p;

            sequence<int32, 5> limited;

            @verbatim (language="comment", text=
                "http://wiki.ros.org/std_msgs")
            std_msgs::msg::Bool q;

            std_msgs::msg::Byte r;

            std_msgs::msg::ByteMultiArray s;

            std_msgs::msg::Char t;

            std_msgs::msg::ColorRGBA u;

            @verbatim (language="comment", text=
                "std_msgs/Duration v")
            std_msgs::msg::Empty w;

            std_msgs::msg::Float32 x;

            std_msgs::msg::Float32MultiArray y;

            std_msgs::msg::Float64 z;

            std_msgs::msg::Float64MultiArray aa;

            std_msgs::msg::Header bb;

            std_msgs::msg::Int16 cc;

            std_msgs::msg::Int16MultiArray dd;

            std_msgs::msg::Int32 ee;

            std_msgs::msg::Int32MultiArray ff;

            std_msgs::msg::Int64 gg;

            std_msgs::msg::Int64MultiArray hh;

            std_msgs::msg::Int8 ii;

            std_msgs::msg::Int8MultiArray jj;

            std_msgs::msg::MultiArrayDimension kk;

            std_msgs::msg::MultiArrayLayout ll;

            std_msgs::msg::String mm;

            @verbatim (language="comment", text=
                "std_msgs/Time nn")
            std_msgs::msg::UInt16 oo;

            std_msgs::msg::UInt16MultiArray pp;

            std_msgs::msg::UInt32 qq;

            std_msgs::msg::UInt32MultiArray rr;

            std_msgs::msg::UInt64 ss;

            std_msgs::msg::UInt64MultiArray tt;

            std_msgs::msg::UInt8 uu;

            std_msgs::msg::UInt8MultiArray vv;

            @verbatim (language="comment", text=
                "default")
            @default (value=40)
            int32 ww;
        };
    };
};"#;

        match parse(input) {
            Ok(definitions) => {
                println!("{:?}", definitions);
            }
            Err(e) => {
                eprintln!("{e}");
                panic!();
            }
        }
    }
}
