use num_bigint::BigInt;
use solang_parser::pt::Loc;
use crate::codegen::cfg::{ASTFunction, ControlFlowGraph, Instr};
use crate::codegen::{Builtin, Expression};
use crate::codegen::vartable::Vartable;
use crate::sema::ast::{ArrayLength, Namespace, StructType, Type};

pub(crate) fn create_preamble(
    parent_func_name: &String,
    ast_func_no: usize,
    ns: &mut Namespace,
) -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new(
        format!("{}::preamble", parent_func_name),
        ASTFunction::None,
    );
    let mut vartab = Vartable::new(ns.next_id);

    let accounts_len = ns.functions[ast_func_no].solana_accounts.borrow().len();
    let accounts = Expression::Builtin(
        Loc::Codegen,
        vec![Type::Array(Box::new(Type::Struct(StructType::AccountInfo)), vec![ArrayLength::Dynamic])],
        Builtin::Accounts,
        vec![]
    );
    let length = Expression::Builtin(
        Loc::Codegen,
        vec![Type::Uint(32)],
        Builtin::ArrayLength,
        vec![accounts.clone()]
    );

    vartab.new_dirty_tracker();
    let in_bounds = cfg.new_basic_block("in_bounds".to_string());
    let out_of_bounds = cfg.new_basic_block("out_of_bounds".to_string());

    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: Expression::More {
                loc: Loc::Codegen,
                signed: false,
                left: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(accounts_len)).into(),
                right: Box::new(length.clone()),
            },
            true_block: out_of_bounds,
            false_block: in_bounds,
        }
    );
    cfg.set_basic_block(out_of_bounds);
    emit_print_message("An account is missing for the transaction".to_string(), &mut vartab, &mut cfg);
    cfg.add(
        &mut vartab,
        Instr::AssertFailure {encoded_args: None}
    );

    let mut validated_block = in_bounds;
    for (account_idx, (account_name, account_data)) in ns.functions[ast_func_no].solana_accounts.borrow().iter().enumerate() {
        cfg.set_basic_block(validated_block);
        let subscript = Expression::Subscript(
            Loc::Codegen,
            Type::Ref(Box::new(Type::Struct(StructType::AccountInfo))),
            Type::Array(Box::new(Type::Struct(StructType::AccountInfo)), vec![ArrayLength::Dynamic]),
            Box::new(accounts.clone()),
            Box::new(
                Expression::NumberLiteral(Loc::Codegen,
                Type::Uint(32),
                    BigInt::from(account_idx)
                )
            ),
        );
        // TODO: Is the load necessary?
        let signer_member = Expression::StructMember(
            Loc::Codegen,
            Type::Ref(Box::new(Type::Bool)),
            Box::new(subscript.clone()),
            5
        );
        let writer_member = Expression::StructMember(
            Loc::Codegen,
            Type::Ref(Box::new(Type::Bool)),
            Box::new(subscript.clone()),
            6
        );

        // let signer_member = Expression::Load(
        //     Loc::Codegen,
        //     Type::Bool,
        //     Box::new(signer_member)
        // );
        // let writer_member = Expression::Load(
        //     Loc::Codegen,
        //     Type::Bool,
        //     Box::new(writer_member)
        // );

        match (account_data.is_signer, account_data.is_writer) {
            (false, false) => (),
            (true, false) => {
                validated_block = cfg.new_basic_block(format!("account_{}_validated", account_idx));
                let failure = cfg.new_basic_block(format!("validation_{}_failed", account_idx));
                cfg.add(
                    &mut vartab,
                    Instr::BranchCond {
                        cond: signer_member,
                        true_block: validated_block,
                        false_block: failure,
                    }
                );
                cfg.set_basic_block(failure);
                emit_print_message(format!("Account '{}' should be a signer", account_name), &mut vartab, &mut cfg);
                cfg.add(
                    &mut vartab,
                    Instr::AssertFailure {
                        encoded_args: None,
                    }
                );
            }
            (false, true) => {
                validated_block = cfg.new_basic_block(format!("account_{}_validated", account_idx));
                let failure = cfg.new_basic_block(format!("validation_{}_failed", account_idx));
                cfg.add(
                    &mut vartab,
                    Instr::BranchCond {
                        cond: writer_member,
                        true_block: validated_block,
                        false_block: failure,
                    }
                );
                cfg.set_basic_block(failure);
                emit_print_message(format!("Account '{}' should be mutable", account_name), &mut vartab, &mut cfg);
                cfg.add(
                    &mut vartab,
                    Instr::AssertFailure {
                        encoded_args: None
                    }
                );
            }
            (true, true) => {
                validated_block = cfg.new_basic_block(format!("account_{}_validated", account_idx));
                let failure = cfg.new_basic_block(format!("validation_{}_failed", account_idx));
                let cond = Expression::BitwiseAnd(
                    Loc::Codegen,
                    Type::Bool,
                    Box::new(signer_member),
                    Box::new(writer_member)
                );
                cfg.add(
                    &mut vartab,
                    Instr::BranchCond {
                        cond,
                        true_block: validated_block,
                        false_block: failure,
                    }
                );
                cfg.set_basic_block(failure);
                emit_print_message(format!("Account '{}' should be a mutable signer", account_name), &mut vartab, &mut cfg);
                cfg.add(
                    &mut vartab,
                    Instr::AssertFailure {encoded_args: None}
                );
            }
        }
    }

    let phis = vartab.pop_dirty_tracker();
    cfg.set_phis(validated_block, phis);
    cfg.set_basic_block(validated_block);
    cfg.add(
        &mut vartab,
        Instr::Return {
            value: vec![]
        }
    );


    cfg
}

fn emit_print_message(
    message: String,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) {
    let expr = Expression::AllocDynamicBytes(
        Loc::Codegen,
        Type::String,
        Box::new(
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(message.len()))
        ),
            Some(message.into_bytes())
    );
    cfg.add(
        vartab,
        Instr::Print {expr}
    );
}