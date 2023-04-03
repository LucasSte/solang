// SPDX-License-Identifier: Apache-2.0

use super::{
    cfg::ReturnCode, expression, Builtin, ControlFlowGraph, Expression, Instr, Options, Type,
    Vartable,
};
use crate::sema::ast::{
    self, ArrayLength, CallTy, ConstructorAnnotation, Function, FunctionAttributes, Namespace,
    StructType,
};
use base58::ToBase58;
use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};
use solang_parser::pt::Loc;

/// Special code for Solana constructors like creating the account
///
/// On Solana, prepare the data account after deploy; ensure the account is
/// large enough and write magic to it to show the account has been deployed.
pub(super) fn solana_deploy(
    func: &Function,
    constructor_args: &[Expression],
    contract_no: usize,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    ns: &Namespace,
    opt: &Options,
) {
    let contract = &ns.contracts[contract_no];

    let program_id = contract.program_id.as_ref();

    if let Some(program_id) = program_id {
        // emit code to check program_id == program_id
        let cond = Expression::Equal(
            Loc::Codegen,
            Box::new(Expression::NumberLiteral(
                Loc::Codegen,
                Type::Address(false),
                BigInt::from_bytes_be(Sign::Plus, program_id),
            )),
            Box::new(Expression::Builtin(
                Loc::Codegen,
                vec![Type::Address(false)],
                Builtin::ProgramId,
                Vec::new(),
            )),
        );

        let id_fail = cfg.new_basic_block("program_id_fail".to_string());

        let id_ok = cfg.new_basic_block("program_id_ok".to_string());

        cfg.add(
            vartab,
            Instr::BranchCond {
                cond,
                true_block: id_ok,
                false_block: id_fail,
            },
        );

        cfg.set_basic_block(id_fail);

        let message = format!("program_id should be {}", program_id.to_base58()).into_bytes();

        let expr = Expression::AllocDynamicBytes(
            Loc::Codegen,
            Type::String,
            Box::new(Expression::NumberLiteral(
                Loc::Codegen,
                Type::Uint(32),
                BigInt::from(message.len()),
            )),
            Some(message),
        );

        cfg.add(vartab, Instr::Print { expr });

        cfg.add(
            vartab,
            Instr::ReturnCode {
                code: ReturnCode::InvalidProgramId,
            },
        );

        cfg.set_basic_block(id_ok);
    }

    // Make sure that the data account is large enough. Read the size of the
    // account via `tx.accounts[0].data.length`.

    // tx.accounts[0]
    let tx_account_0 = Expression::Subscript(
        Loc::Codegen,
        Type::Struct(StructType::AccountInfo),
        Type::Array(
            Type::Struct(StructType::AccountInfo).into(),
            vec![ArrayLength::Dynamic],
        ),
        Expression::Builtin(
            Loc::Codegen,
            vec![Type::Array(
                Type::Struct(StructType::AccountInfo).into(),
                vec![ArrayLength::Dynamic],
            )],
            Builtin::Accounts,
            vec![],
        )
        .into(),
        Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::zero()).into(),
    );

    // .data.length
    let account_length = Expression::Builtin(
        Loc::Codegen,
        vec![Type::Uint(32)],
        Builtin::ArrayLength,
        vec![Expression::StructMember(
            Loc::Codegen,
            Type::DynamicBytes,
            tx_account_0.into(),
            2,
        )],
    );

    let account_data_var = vartab.temp_name("data_length", &Type::Uint(32));

    cfg.add(
        vartab,
        Instr::Set {
            loc: Loc::Codegen,
            res: account_data_var,
            expr: account_length,
        },
    );

    let account_length = Expression::Variable(Loc::Codegen, Type::Uint(32), account_data_var);

    let account_no_data = Expression::Equal(
        Loc::Codegen,
        account_length.clone().into(),
        Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), 0.into()).into(),
    );

    let account_exists = cfg.new_basic_block("account_exists".into());
    let create_account = cfg.new_basic_block("create_account".into());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: account_no_data,
            true_block: create_account,
            false_block: account_exists,
        },
    );

    cfg.set_basic_block(account_exists);

    let is_enough = Expression::MoreEqual {
        loc: Loc::Codegen,
        signed: false,
        left: account_length.into(),
        right: Expression::NumberLiteral(
            Loc::Codegen,
            Type::Uint(32),
            contract.fixed_layout_size.clone(),
        )
        .into(),
    };

    let account_ok = cfg.new_basic_block("account_ok".into());
    let not_enough = cfg.new_basic_block("not_enough".into());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: is_enough,
            true_block: account_ok,
            false_block: not_enough,
        },
    );

    cfg.set_basic_block(not_enough);

    cfg.add(
        vartab,
        Instr::ReturnCode {
            code: ReturnCode::AccountDataTooSmall,
        },
    );

    cfg.set_basic_block(create_account);

    // The expressions in the @payer, @seed, @bump, and @space have been resolved in the constructors
    // context, so any variables will be bound to that vartable. Only the parameters are visible
    // when these were resolved; simply copy the decoded constructor arguments into the right variables.
    for (i, arg) in func.get_symbol_table().arguments.iter().enumerate() {
        if let Some(arg) = arg {
            let param = &func.params[i];

            vartab.add_known(*arg, param.id.as_ref().unwrap(), &param.ty);

            cfg.add(
                vartab,
                Instr::Set {
                    loc: Loc::Codegen,
                    res: *arg,
                    expr: constructor_args[i].clone(),
                },
            );
        }
    }

    if let Some(ConstructorAnnotation::Payer(_, account_name)) = func
        .annotations
        .iter()
        .find(|tag| matches!(tag, ConstructorAnnotation::Payer(..)))
    {
        let metas_ty = Type::Array(
            Box::new(Type::Struct(StructType::AccountMeta)),
            vec![ArrayLength::Fixed(BigInt::from(2))],
        );

        let metas = vartab.temp_name("metas", &metas_ty);
        let account_index = func.solana_accounts.borrow().get_index_of(account_name).unwrap();

        let accounts = Expression::Builtin(
            Loc::Codegen,
            vec![Type::Array(Box::new(Type::Struct(StructType::AccountInfo)), vec![ArrayLength::Dynamic])],
                Builtin::Accounts,
            vec![]
        );

        // TODO: This can be simplified
        let subscript = Expression::Subscript(
            Loc::Codegen,
            Type::Ref(Box::new(Type::Struct(StructType::AccountInfo))),
            Type::Array(Box::new(Type::Struct(StructType::AccountInfo)), vec![ArrayLength::Dynamic]),
            Box::new(accounts.clone()),
            Box::new(
                Expression::NumberLiteral(Loc::Codegen,
                                          Type::Uint(32),
                                          BigInt::from(account_index)
                )
            ),
        );

        let payer_key = Expression::StructMember(
            Loc::Codegen,
            Type::Ref(Box::new(Type::Bool)),
            Box::new(subscript.clone()),
            0
        );

        cfg.add(
            vartab,
            Instr::Set {
                loc: Loc::Codegen,
                res: metas,
                expr: Expression::ArrayLiteral(
                    Loc::Codegen,
                    metas_ty.clone(),
                    vec![2],
                    vec![
                        Expression::StructLiteral(
                            Loc::Codegen,
                            Type::Struct(StructType::AccountMeta),
                            vec![
                                Expression::GetRef(
                                    Loc::Codegen,
                                    Type::Address(false),
                                    Box::new(payer_key),
                                ),
                                Expression::BoolLiteral(Loc::Codegen, true),
                                Expression::BoolLiteral(Loc::Codegen, true),
                            ],
                        ),
                        Expression::StructLiteral(
                            Loc::Codegen,
                            Type::Struct(StructType::AccountMeta),
                            vec![
                                Expression::GetRef(
                                    Loc::Codegen,
                                    Type::Address(false),
                                    Box::new(Expression::Builtin(
                                        Loc::Codegen,
                                        vec![Type::Address(false)],
                                        Builtin::GetAddress,
                                        vec![],
                                    )),
                                ),
                                Expression::BoolLiteral(Loc::Codegen, true),
                                Expression::BoolLiteral(Loc::Codegen, true),
                            ],
                        ),
                    ],
                ),
            },
        );

        // Calculate minimum balance for rent-excempt
        let (space, lamports) = if let Some(ConstructorAnnotation::Space(space_expr)) = func
            .annotations
            .iter()
            .find(|tag| matches!(tag, ConstructorAnnotation::Space(..)))
        {
            let space_var = vartab.temp_name("space", &Type::Uint(64));
            let expr = expression(space_expr, cfg, contract_no, None, ns, vartab, opt);

            cfg.add(
                vartab,
                Instr::Set {
                    loc: Loc::Codegen,
                    res: space_var,
                    expr,
                },
            );

            let space = Expression::Variable(Loc::Codegen, Type::Uint(64), space_var);

            // https://github.com/solana-labs/solana/blob/718f433206c124da85a8aa2476c0753f351f9a28/sdk/program/src/rent.rs#L78-L82
            let lamports = Expression::Multiply(
                Loc::Codegen,
                Type::Uint(64),
                false,
                Expression::Add(
                    Loc::Codegen,
                    Type::Uint(64),
                    false,
                    space.clone().into(),
                    Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), 128.into()).into(),
                )
                .into(),
                Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), BigInt::from(3480 * 2))
                    .into(),
            );

            (space, lamports)
        } else {
            let space_runtime_constant = contract.fixed_layout_size.to_u64().unwrap();

            // https://github.com/solana-labs/solana/blob/718f433206c124da85a8aa2476c0753f351f9a28/sdk/program/src/rent.rs#L78-L82
            let lamports_runtime_constant = (128 + space_runtime_constant) * 3480 * 2;

            (
                Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Uint(64),
                    space_runtime_constant.into(),
                ),
                Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Uint(64),
                    lamports_runtime_constant.into(),
                ),
            )
        };

        let instruction_var = vartab.temp_name("instruction", &Type::DynamicBytes);
        let instruction = Expression::Variable(Loc::Codegen, Type::DynamicBytes, instruction_var);

        // The CreateAccount instruction is 52 bytes (4 + 8 + 8 + 32)
        let instruction_size = 52;

        cfg.add(
            vartab,
            Instr::Set {
                loc: Loc::Codegen,
                res: instruction_var,
                expr: Expression::AllocDynamicBytes(
                    Loc::Codegen,
                    Type::DynamicBytes,
                    Expression::NumberLiteral(
                        Loc::Codegen,
                        Type::Uint(32),
                        instruction_size.into(),
                    )
                    .into(),
                    None,
                ),
            },
        );

        // instruction CreateAccount
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: instruction.clone(),
                value: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(0)),
                offset: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(0)),
            },
        );

        // lamports
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: instruction.clone(),
                value: lamports,
                offset: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(4)),
            },
        );

        // space
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: instruction.clone(),
                value: space,
                offset: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(12)),
            },
        );

        // owner
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: instruction.clone(),
                value: if let Some(program_id) = program_id {
                    Expression::NumberLiteral(
                        Loc::Codegen,
                        Type::Address(false),
                        BigInt::from_bytes_be(Sign::Plus, program_id),
                    )
                } else {
                    Expression::Builtin(
                        Loc::Codegen,
                        vec![Type::Address(false)],
                        Builtin::ProgramId,
                        vec![],
                    )
                },
                offset: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::from(20)),
            },
        );

        // seeds
        let mut seeds = Vec::new();

        for note in &func.annotations {
            match note {
                ConstructorAnnotation::Seed(seed) => {
                    seeds.push(expression(seed, cfg, contract_no, None, ns, vartab, opt));
                }
                ConstructorAnnotation::Bump(bump) => {
                    let expr = ast::Expression::Cast {
                        loc: Loc::Codegen,
                        to: Type::Slice(Type::Bytes(1).into()),
                        expr: ast::Expression::BytesCast {
                            loc: Loc::Codegen,
                            to: Type::DynamicBytes,
                            from: Type::Bytes(1),
                            expr: bump.clone().into(),
                        }
                        .into(),
                    };

                    seeds.push(expression(&expr, cfg, contract_no, None, ns, vartab, opt));
                }
                _ => (),
            }
        }

        let seeds = if !seeds.is_empty() {
            let ty = Type::Array(
                Box::new(Type::Slice(Box::new(Type::Bytes(1)))),
                vec![ArrayLength::Fixed(seeds.len().into())],
            );

            let address_seeds =
                Expression::ArrayLiteral(Loc::Codegen, ty, vec![seeds.len() as u32], seeds);

            let ty = Type::Array(
                Box::new(Type::Slice(Box::new(Type::Slice(Box::new(Type::Bytes(1)))))),
                vec![ArrayLength::Fixed(1.into())],
            );

            Some(Expression::ArrayLiteral(
                Loc::Codegen,
                ty,
                vec![1],
                vec![address_seeds],
            ))
        } else {
            None
        };

        cfg.add(
            vartab,
            Instr::ExternalCall {
                success: None,
                seeds,
                address: Some(Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Address(false),
                    BigInt::from(0),
                )), // SystemProgram 11111111111111111111111111111111
                accounts: Some(Expression::Variable(Loc::Codegen, metas_ty, metas)),
                payload: instruction,
                value: Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), BigInt::from(0)),
                gas: Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), BigInt::from(0)),
                callty: CallTy::Regular,
                contract_function_no: None,
            },
        );

        cfg.add(vartab, Instr::Branch { block: account_ok });
    } else {
        cfg.add(
            vartab,
            Instr::ReturnCode {
                code: ReturnCode::AccountDataTooSmall,
            },
        );
    }

    cfg.set_basic_block(account_ok);

    // Write contract magic number to offset 0
    cfg.add(
        vartab,
        Instr::SetStorage {
            ty: Type::Uint(32),
            value: Expression::NumberLiteral(
                Loc::Codegen,
                Type::Uint(64),
                BigInt::from(contract.selector()),
            ),
            storage: Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), BigInt::zero()),
        },
    );

    // Calculate heap offset
    let fixed_fields_size = contract.fixed_layout_size.to_u64().unwrap();

    // align on 8 byte boundary (round up to nearest multiple of 8)
    let heap_offset = (fixed_fields_size + 7) & !7;

    // Write heap offset to 12
    cfg.add(
        vartab,
        Instr::SetStorage {
            ty: Type::Uint(32),
            value: Expression::NumberLiteral(
                Loc::Codegen,
                Type::Uint(64),
                BigInt::from(heap_offset),
            ),
            storage: Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), BigInt::from(12)),
        },
    );
}
