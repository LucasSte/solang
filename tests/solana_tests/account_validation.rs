use num_bigint::BigInt;
use crate::borsh_encoding::BorshToken;
use crate::{Account, AccountMeta, AccountState, build_solidity, Pubkey};

#[test]
fn missing_account() {
    let mut vm = build_solidity(
        r#"
contract test {
    @reader(acc1)
    function sum(int256 a, int256 b) public view returns (int256) {
        if (tx.accounts.acc1.is_writable) {
            return a-b;
        }
        return a+b;
    }

    @signer(acc1)
    @mutable(acc2)
    function add2(int256 a, int256 b) public view returns (int256) {
        if (tx.accounts.acc1.is_writable && tx.accounts.acc2.is_signer) {
            return a-b;
        }
        return a+b;
    }

}
        "#
    );
    vm.constructor(&[]);

    let args = vec![
        BorshToken::Int {
            width: 256,
            value: BigInt::from(53u8)
        },
        BorshToken::Int {
            width: 256,
            value: BigInt::from(50u8)
        }
    ];

    let accounts = vec![
        AccountMeta {
            pubkey: Pubkey(vm.stack[0].data),
            is_writable: false,
            is_signer: false,
        }
    ];

    let results = vm.function_must_fail_with_metas("sum", &accounts, &args);
    assert_eq!(results.unwrap(), 0x100000000);
    assert_eq!(vm.logs, "An account is missing for the transaction");

    vm.account_data.insert([0; 32], AccountState {
        data: vec![],
        owner: None,
        lamports: 0,
    });
    let accounts = vec![
        AccountMeta {
            pubkey: Pubkey(vm.stack[0].data),
            is_writable: false,
            is_signer: false,
        },
        AccountMeta {
            pubkey: Pubkey([0; 32]),
            is_writable: false,
            is_signer: false,
        }
    ];
    vm.logs.clear();
    let results = vm.function_must_fail_with_metas("add2", &accounts, &args);
    assert_eq!(results.unwrap(), 0x100000000);
    assert_eq!(vm.logs, "An account is missing for the transaction");
}

#[test]
fn account_signer() {
    let mut vm = build_solidity(
        r#"
    contract test {

    @signer(acc1)
    function add(int256 a, int256 b) public view returns (int256) {
        if (tx.accounts.acc1.is_writable) {
            return a-b;
        }
        return a+b;
    }

    }
        "#
    );
    vm.constructor(&[]);

    let args = vec![
        BorshToken::Int {
            width: 256,
            value: BigInt::from(53u8)
        },
        BorshToken::Int {
            width: 256,
            value: BigInt::from(50u8)
        }
    ];

    let mut accounts = vec![
        AccountMeta {
            pubkey: Pubkey([5; 32]),
            is_writable: true,
            is_signer: true,
        },
        AccountMeta {
            pubkey: Pubkey(vm.stack[0].data),
            is_writable: false,
            is_signer: false,
        },
    ];

    vm.account_data.insert([5; 32], AccountState {
        data: vec![],
        owner: None,
        lamports: 0,
    });

    // let results = vm.function_must_fail_with_metas("add", &accounts, &args);
    // assert_eq!(results.unwrap(), 0x100000000);
    // assert_eq!(vm.logs, "Account 'acc1' should be a signer");

    //accounts[1].is_signer = true;
    let results = vm.function_metas("add", &accounts, &args).unwrap();
    assert_eq!(results, BorshToken::Int {
        width: 256,
        value: BigInt::from(103)
    });
}

#[test]
fn account_not_mutable() {
    let mut vm = build_solidity(
        r#"
    contract test {

    @mutable(acc1)
    function add(int256 a, int256 b) public view returns (int256) {
        if (tx.accounts.acc1.is_writable) {
            return a-b;
        }
        return a+b;
    }

    }
        "#
    );
    vm.constructor(&[]);

    let args = vec![
        BorshToken::Int {
            width: 256,
            value: BigInt::from(53u8)
        },
        BorshToken::Int {
            width: 256,
            value: BigInt::from(50u8)
        }
    ];

    let accounts = vec![
        AccountMeta {
            pubkey: Pubkey(vm.stack[0].data),
            is_writable: false,
            is_signer: false,
        },
        AccountMeta {
            pubkey: Pubkey([5; 32]),
            is_writable: false,
            is_signer: true,
        }
    ];

    vm.account_data.insert([5; 32], AccountState {
        data: vec![],
        owner: None,
        lamports: 0,
    });

    let results = vm.function_must_fail_with_metas("add", &accounts, &args);
    assert_eq!(results.unwrap(), 0x100000000);
    assert_eq!(vm.logs, "Account 'acc1' should be mutable");
}


#[test]
fn account_not_mutable_signer() {
    let mut vm = build_solidity(
        r#"
    contract test {

    @mutableSigner(acc1)
    function add(int256 a, int256 b) public view returns (int256) {
        if (tx.accounts.acc1.is_writable) {
            return a-b;
        }
        return a+b;
    }

    }
        "#
    );
    vm.constructor(&[]);

    let args = vec![
        BorshToken::Int {
            width: 256,
            value: BigInt::from(53u8)
        },
        BorshToken::Int {
            width: 256,
            value: BigInt::from(50u8)
        }
    ];

    let mut accounts = vec![
        AccountMeta {
            pubkey: Pubkey(vm.stack[0].data),
            is_writable: false,
            is_signer: false,
        },
        AccountMeta {
            pubkey: Pubkey([5; 32]),
            is_writable: false,
            is_signer: false,
        }
    ];

    vm.account_data.insert([5; 32], AccountState {
        data: vec![],
        owner: None,
        lamports: 0,
    });

    let results = vm.function_must_fail_with_metas("add", &accounts, &args);
    assert_eq!(results.unwrap(), 0x100000000);
    assert_eq!(vm.logs, "Account 'acc1' should be a mutable signer");


    vm.logs.clear();
    accounts[1].is_signer = true;
    let results = vm.function_must_fail_with_metas("add", &accounts, &args);
    assert_eq!(results.unwrap(), 0x100000000);
    assert_eq!(vm.logs, "Account 'acc1' should be a mutable signer");

    vm.logs.clear();
    accounts[1].is_signer = false;
    accounts[1].is_writable = true;
    let results = vm.function_must_fail_with_metas("add", &accounts, &args);
    assert_eq!(results.unwrap(), 0x100000000);
    assert_eq!(vm.logs, "Account 'acc1' should be a mutable signer");
}