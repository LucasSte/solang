// RUN: --target solana --emit cfg

contract test {

    @reader(acc1)
    function sum(int256 a, int256 b) public view returns (int256) {
        // BEGIN-CHECK: test::test::function::sum__int256_int256
        // CHECK: block0: # entry
	    // CHECK: = call test::test::function::sum__int256_int256::preamble

        if (tx.accounts.acc1.is_writable) {
            return a-b;
        }
        return a+b;
    }

    // BEGIN-CHECK: test::test::function::sum__int256_int256::preamble
    // CHECK: block0: # entry
	// CHECK: branchcond (unsigned more uint32 2 > (builtin ArrayLength ((builtin Accounts ())))), block2, block1
    // CHECK: block1: # in_bounds
	// CHECK: return
    // CHECK: block2: # out_of_bounds
	// CHECK: print (alloc string uint32 41 "An account is missing for the transaction")
	// CHECK: assert-failure

    @signer(acc1)
    function add(int256 a, int256 b) public view returns (int256) {
        // BEGIN-CHECK: test::test::function::add__int256_int256
        // CHECK: block0: # entry
	    // CHECK: = call test::test::function::add__int256_int256::preamble
        if (tx.accounts.acc1.is_writable) {
            return a-b;
        }
        return a+b;
    }

    // BEGIN-CHECK: test::test::function::add__int256_int256::preamble
    // CHECK: block0: # entry
	// CHECK: branchcond (unsigned more uint32 2 > (builtin ArrayLength ((builtin Accounts ())))), block2, block1
    // CHECK: block1: # in_bounds
	// CHECK: branchcond (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1]) field 5), block3, block4
    // CHECK: block2: # out_of_bounds
	// CHECK: print (alloc string uint32 41 "An account is missing for the transaction")
	// CHECK: assert-failure
    // CHECK: block3: # account_1_validated
	// CHECK: return
    // CHECK: block4: # validation_1_failed
	// CHECK: print (alloc string uint32 33 "Account \'acc1\' should be a signer")
	// CHECK: assert-failure

    @signer(acc1)
    @mutable(acc2)
    function add2(int256 a, int256 b) public view returns (int256) {
        // BEGIN-CHECK: test::test::function::add2__int256_int256
        // CHECK: block0: # entry
	    // CHECK: = call test::test::function::add2__int256_int256::preamble
        if (tx.accounts.acc1.is_writable && tx.accounts.acc2.is_signer) {
            return a-b;
        }
        return a+b;
    }

    // BEGIN-CHECK: test::test::function::add2__int256_int256::preamble
    // CHECK: block0: # entry
	// CHECK: branchcond (unsigned more uint32 3 > (builtin ArrayLength ((builtin Accounts ())))), block2, block1
    // CHECK: block1: # in_bounds
	// CHECK: branchcond (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1]) field 5), block3, block4
    // CHECK: block2: # out_of_bounds
	// CHECK: print (alloc string uint32 41 "An account is missing for the transaction")
	// CHECK: assert-failure
    // CHECK: block3: # account_1_validated
	// CHECK: branchcond (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 2]) field 6), block5, block6
    // CHECK: block4: # validation_1_failed
	// CHECK: print (alloc string uint32 33 "Account \'acc1\' should be a signer")
	// CHECK: assert-failure
    // CHECK: block5: # account_2_validated
	// CHECK: return
    // CHECK: block6: # validation_2_failed
	// CHECK: print (alloc string uint32 32 "Account \'acc2\' should be mutable")
	// CHECK: assert-failure

    @mutableSigner(acc3)
    function add3(int256 a, int256 b) public view returns (int256) {
        // BEGIN-CHECK: test::test::function::add3__int256_int256
        // CHECK: block0: # entry
	    // CHECK: = call test::test::function::add3__int256_int256::preamble
        if (tx.accounts.acc3.is_writable && tx.accounts.acc3.is_signer) {
            return a-b;
        }
        return a+b;
    }

    // BEGIN-CHECK: test::test::function::add3__int256_int256::preamble
    // CHECK: block0: # entry
	// CHECK: branchcond (unsigned more uint32 2 > (builtin ArrayLength ((builtin Accounts ())))), block2, block1
    // CHECK: block1: # in_bounds
	// CHECK: branchcond ((struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1]) field 5) & (struct (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1]) field 6)), block3, block4
    // CHECK: block2: # out_of_bounds
	// CHECK: print (alloc string uint32 41 "An account is missing for the transaction")
	// CHECK: assert-failure
    // CHECK: block3: # account_1_validated
	// CHECK: return
    // CHECK: block4: # validation_1_failed
	// CHECK: print (alloc string uint32 41 "Account \'acc3\' should be a mutable signer")
	// CHECK: assert-failure
}
