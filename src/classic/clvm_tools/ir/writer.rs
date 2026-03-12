use std::borrow::Borrow;
use std::rc::Rc;

use to_binary::BinaryString;

use crate::classic::clvm::__type_compatibility__::{Bytes, BytesFromType, Stream};
use crate::classic::clvm::casts::{bigint_from_bytes, TConvertOption};

use crate::classic::clvm_tools::ir::r#type::{IRRepr, NEW_BIT_CONSTANTS};

#[derive(Debug)]
enum IROutputState {
    Start(Rc<IRRepr>),
    MaybeSep(Rc<IRRepr>),
    ListOf(Rc<IRRepr>),
    DotThen(Rc<IRRepr>),
    EndParen,
}

#[derive(Debug)]
struct IROutputIterator {
    state: Vec<IROutputState>,
    language_flags: u32
}

impl IROutputIterator {
    fn new(ir_sexp: Rc<IRRepr>, flags: u32) -> IROutputIterator {
        IROutputIterator {
            state: vec![IROutputState::Start(ir_sexp)],
            language_flags: flags,
        }
    }
}

impl Iterator for IROutputIterator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state.pop() {
                None => {
                    return None;
                }
                Some(IROutputState::EndParen) => {
                    return Some(")".to_string());
                }
                Some(IROutputState::Start(v)) => match v.borrow() {
                    IRRepr::Cons(l, r) => {
                        self.state.push(IROutputState::ListOf(Rc::new(IRRepr::Cons(
                            l.clone(),
                            r.clone(),
                        ))));
                        return Some("(".to_string());
                    }
                    IRRepr::Null => {
                        return Some("()".to_string());
                    }
                    IRRepr::Quotes(q) => {
                        return Some(q.to_formal_string());
                    }
                    IRRepr::Int(i, signed) => {
                        let opts = TConvertOption { signed: *signed };
                        return Some(bigint_from_bytes(i, Some(opts)).to_string());
                    }
                    IRRepr::Hex(h) => {
                        return Some("0x".to_string() + &h.hex());
                    }
                    IRRepr::Octal(o) => {
                        if (self.language_flags & NEW_BIT_CONSTANTS) != 0 {
                            let mut buffer: u16 = 0;
                            let mut buffer_bits: usize = 0;
                            let mut output_vec = Vec::new();

                            let spill_bits = |buffer: &mut u16, buffer_bits: &mut usize, output_vec: &mut Vec<u8>, while_zero: bool| {
                                while *buffer_bits >= 3 && (*buffer != 0 || while_zero) {
                                    output_vec.push(b'0' + (*buffer & 7) as u8);
                                    *buffer >>= 3;
                                    *buffer_bits -= 3;
                                }
                            };

                            for byte in o.data().iter() {
                                buffer <<= 8;
                                buffer |= *byte as u16;
                                buffer_bits += 8;
                                spill_bits(&mut buffer, &mut buffer_bits, &mut output_vec, true);
                            }

                            spill_bits(&mut buffer, &mut buffer_bits, &mut output_vec, false);

                            return Some("0o".to_string() + &String::from_utf8_lossy(&output_vec));
                        }

                        return Some("0x".to_string() + &o.hex());
                    }
                    IRRepr::Binary(b) => {
                        if (self.language_flags & NEW_BIT_CONSTANTS) != 0 {
                            let bdata: &[u8] = &b.data();
                            return Some("0b".to_string() + &BinaryString::from(bdata).to_string());
                        }

                        return Some("0x".to_string() + &b.hex());
                    }
                    IRRepr::Symbol(s) => {
                        return Some(s.to_string());
                    }
                },
                Some(IROutputState::MaybeSep(sub)) => match sub.borrow() {
                    IRRepr::Null => {
                        self.state.push(IROutputState::EndParen);
                    }
                    _ => {
                        self.state.push(IROutputState::ListOf(sub.clone()));
                        return Some(" ".to_string());
                    }
                },
                Some(IROutputState::ListOf(v)) => match v.borrow() {
                    IRRepr::Cons(l, r) => {
                        self.state.push(IROutputState::MaybeSep(r.clone()));
                        self.state.push(IROutputState::Start(l.clone()));
                    }
                    IRRepr::Null => {
                        self.state.push(IROutputState::EndParen);
                    }
                    _ => {
                        self.state.push(IROutputState::EndParen);
                        self.state.push(IROutputState::DotThen(v.clone()));
                        return Some(". ".to_string());
                    }
                },
                Some(IROutputState::DotThen(v)) => match v.borrow() {
                    IRRepr::Cons(l, r) => {
                        self.state.push(IROutputState::ListOf(r.clone()));
                        self.state.push(IROutputState::Start(l.clone()));
                    }
                    _ => {
                        self.state.push(IROutputState::Start(v.clone()));
                    }
                },
            }
        }
    }
}

pub fn write_ir_to_stream(ir_sexp: Rc<IRRepr>, f: &mut Stream, language_flags: u32) {
    for b in IROutputIterator::new(ir_sexp, language_flags) {
        f.write(Bytes::new(Some(BytesFromType::String(b))));
    }
}

pub fn write_ir(ir_sexp: Rc<IRRepr>, language_flags: u32) -> String {
    let mut s = Stream::new(None);
    write_ir_to_stream(ir_sexp, &mut s, language_flags);
    s.get_value().decode()
}
