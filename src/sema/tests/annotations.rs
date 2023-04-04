use crate::sema::tests::parse;
use crate::Target;

#[cfg(test)]

#[test]
fn invalid_parameters() {
    let src = r#"
    contract test {

    @signer(1acc1)
    function sum(int256 a, int256 b) public returns (int256) {
        if (tx.accounts.acc1.key == address(this)) {
            return a-b;
        }
        return a+b;
    }

}
    "#;
    let ns = parse(src, Target::Solana);

    assert!(ns.diagnostics.contains_message(
        "invalid parameter for annotation"
    ));

    let src = r#"
        contract test {

    @signer(25)
    function sum(int256 a, int256 b) public returns (int256) {
        if (tx.accounts.acc1.key == address(this)) {
            return a-b;
        }
        return a+b;
    }

}
    "#;
    let ns = parse(src, Target::Solana);
    assert!(ns.diagnostics.contains_message("invalid parameter for annotation"));
}

#[test]
fn account_not_declared() {
    let src = r#"
    contract test {

    @signer(acc2)
    function sum(int256 a, int256 b) public returns (int256) {
        if (tx.accounts.acc1.key == address(this)) {
            return a-b;
        }
        return a+b;
    }

}
    "#;
    let ns = parse(src, Target::Solana);
    assert!(ns.diagnostics.contains_message("account 'acc1' not declared"));
}

#[test]
fn all_accounts_ok() {
    let src = r#"
    contract test {

    @signer(acc2)
    function sum(int256 a, int256 b) public view returns (int256) {
        if (tx.accounts.acc2.key == address(this)) {
            return a-b;
        }
        return a+b;
    }

    @mutable(acc1)
    function test_account(address ss) public view returns (bool) {
        if(ss == tx.accounts.acc1.key) {
            return true;
        }

        return false;
    }

    @reader(acc4)
    @mutableSigner(acc3)
    function test_mutable_signer(address ss) public view returns (bool) {
        if(ss == tx.accounts.acc3.key && tx.accounts.acc4.is_signer) {
            return true;
        }
        return false;
    }

    // pure functions do not require that data account, so the name is allwoed here.
    @reader(dataAccount)
    function test_pure_function(uint a, uint b) public pure returns (uint) {
        return a+b;
    }

}
    "#;
    let ns = parse(src, Target::Solana);
    assert_eq!(ns.diagnostics.len(), 1);
    assert!(ns.diagnostics.contains_message("found contract 'test'"));
}

#[test]
fn data_account_reserved_view_function() {
    let src = r#"
    contract test {

    @mutable(dataAccount)
    function add(int256 a, int256 b) public view returns (int256) {
        if (tx.accounts[0].is_signer) {
            return a-b;
        }
        return a+b;
    }

}
    "#;
    let ns = parse(src, Target::Solana);
    assert!(ns.diagnostics.contains_message("'dataAccount' is a reserved account name for the contract's data account"));
}

#[test]
fn data_account_reserved() {
    let src = r#"
    contract test {
    int b;

    @mutable(dataAccount)
    function add(int256 a, int256 b) public returns (int256) {
        if (tx.accounts[0].is_signer) {
            return a-b;
        }
        b = a+b;
        return a+b;
    }

}
    "#;
    let ns = parse(src, Target::Solana);
    assert!(ns.diagnostics.contains_message("'dataAccount' is a reserved account name for the contract's data account"));
}

#[test]
fn duplicate_account_declaration() {
    let src = r#"
    contract test {

    @signer(acc2)
    @mutable(acc2)
    function add(int256 a, int256 b) public view returns (int256) {
        if (tx.accounts[0].is_signer) {
            return a-b;
        }
        return a+b;
    }

}
    "#;
    let ns = parse(src, Target::Solana);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "account 'acc2' already defined");
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(errors[0].notes[0].message, "previous definition here");
}

// TODO:
// 4. Write integrations tests that verify if the accounts are properly deserialized
// 5. Write the docs about how to use this new feature
// 6. Modify the docs about the @payer annotation
// 7. The syntax needs changing
// 8. Fix graphviz
// 9. Fix language server