// `SyncLock` implemented in Promela for verification by SPIN model checker
// (http://spinroot.com/spin/whatispin.html)
//
// Usage:
//
//     $ spin -a lock-stdimp.pml && gcc -o pan -O3 pan.c && ./pan
//
//     If the verification fails, analyze the generated trail file by:
//     $ spin -t -r -s -l -g -c lock-stdimp.pml
//

/// `SyncLock::count`
int count = 0;

/// The park token for the `main_lock` process
chan parker = [1] of { bit };

int PARKED_FLAG = 16;
int EXCLUSIVE_FLAG = 8;

// -------------------------------------------------------------------------
// Atomic operations

inline fetch_add(var, operand, out_old_value) {
    d_step { out_old_value = var; var = var + operand; }
}
inline fetch_sub(var, operand, out_old_value) {
    d_step { out_old_value = var; var = var - operand; }
}
inline compare_exchange(var, old_value, new_value, out_old_value, out_success) {
    d_step {
        out_old_value = var
        if
        ::  (var == old_value) ->
            out_success = true
            var = new_value
        ::  else ->
            out_success = false
        fi
    }
}
inline unpark() {
    if
    :: len(parker) == 0 -> parker!0
    :: else -> skip
    fi
}

// -------------------------------------------------------------------------
// `SyncLock`

inline lock_shared() {
    printf("lock_shared\n");

    int old_count;
    fetch_add(count, 1, old_count);

    assert((old_count & PARKED_FLAG) == 0);

    if
    ::  (old_count < EXCLUSIVE_FLAG - 2) -> skip
    ::  else -> lock_shared_slow(old_count)
    fi
}

inline try_lock_shared(out_success) {
    printf("try_lock_shared\n");

    int old_count;
    fetch_add(count, 1, old_count);

    assert((old_count & PARKED_FLAG) == 0);

    if
    ::  (old_count < EXCLUSIVE_FLAG - 2) -> out_success = true
    ::  else ->
        fetch_sub(count, 1, old_count)
        out_success = false
    fi
}

inline lock_shared_slow(old_count) {
    printf("lock_shared_slow\n");

    // lock counter overflow
    assert(old_count != EXCLUSIVE_FLAG - 2);

    assert(old_count == EXCLUSIVE_FLAG);

    bool cx_success;
    int old_count2;
    compare_exchange(count, EXCLUSIVE_FLAG + 1, PARKED_FLAG + EXCLUSIVE_FLAG,
        old_count2, cx_success);

    bit _unused;

    if
    ::  (cx_success) ->
        // Will be unparked when the exclusive lock is released
        do
        :: parker?_unused ->
            // Check for spurious wake ups
            if
            ::  count == 0 -> break
            ::  else -> skip
            fi
        od
        count = 1;
    ::  (!cx_success) ->
        // It was unlocked before the `compare_exchange`
        assert(old_count2 == 1);
    fi
}

inline unlock_shared() {
    printf("unlock_shared\n");

    int old_count;
    fetch_sub(count, 1, old_count);

    if
    ::  (old_count == PARKED_FLAG + 1) ->
        // The creator thread is parked in `lock_exclusive_slow`
        count = 0;
        unpark();
    ::  else ->
        assert((old_count & EXCLUSIVE_FLAG) == 0);
        assert((old_count & ~PARKED_FLAG) > 0);
    fi
}

inline lock_exclusive() {
    printf("lock_exclusive\n");

    int old_count;
    old_count = count
    if
    ::  (old_count == 0) -> count = EXCLUSIVE_FLAG;
    ::  else -> lock_exclusive_slow(old_count);
    fi
}

inline try_lock_exclusive(out_success) {
    printf("try_lock_exclusive\n");

    if
    ::  (count == 0) ->
        count = EXCLUSIVE_FLAG;
        out_success = true;
    ::  else ->
        out_success = false;
    fi
}

inline lock_exclusive_slow(old_count) {
    printf("lock_exclusive_slow\n");

    int old_count2;
    bit _unused;

    // Park the current thread
    fetch_add(count, PARKED_FLAG, old_count2);

    if
    ::  (old_count2 == 0) ->
        // It was unlocked before the `fetch_add`
        skip
    ::  else ->
        // Will be unparked when the exclusive or shared lock(s) are
        // released
        do
        :: parker?_unused ->
            // Check for spurious wake ups
            if
            ::  count == 0 -> break
            ::  else -> skip
            fi
        od
    fi

    count = EXCLUSIVE_FLAG;
}

inline unlock_exclusive() {
    printf("unlock_exclusive\n");

    int old_count;
    fetch_sub(count, EXCLUSIVE_FLAG, old_count);
    assert(
        old_count == EXCLUSIVE_FLAG ||
        // a portion of `lock_shared` and `try_lock_shared`
        old_count == EXCLUSIVE_FLAG + 1 ||
        // parking of `lock_shared_slow` or `lock_exclusive_slow`
        old_count == (PARKED_FLAG | EXCLUSIVE_FLAG)
    );

    if
    ::  (old_count == (PARKED_FLAG | EXCLUSIVE_FLAG)) ->
        // The creator thread is parked in `lock_shared_slow` or
        // `lock_exclusive_slow`
        count = 0;
        unpark();
    ::  else -> skip
    fi
}

// -------------------------------------------------------------------------
// Lock validator

int count_shadow = 0;

inline log_lock_shared() {
    d_step {
        assert(count_shadow != EXCLUSIVE_FLAG);
        count_shadow = count_shadow + 1;
    }
}

inline log_lock_exclusive() {
    d_step {
        assert(count_shadow == 0);
        count_shadow = EXCLUSIVE_FLAG
    }
}

inline log_unlock_shared() {
    d_step {
        assert(count_shadow != EXCLUSIVE_FLAG);
        assert(count_shadow > 0);
        count_shadow = count_shadow - 1;
    }
}

inline log_unlock_exclusive() {
    d_step {
        assert(count_shadow == EXCLUSIVE_FLAG);
        count_shadow = 0;
    }
}

// -------------------------------------------------------------------------
// Test bench

#define NPROC 4

chan lock_guard_sender = [NPROC] of { bool };

active proctype main_lock() {
    // The number of acquired locks.
    int i = 0;
    do
    ::  i < NPROC ->
        bool success;

        if
        :: true ->
            lock_shared();
            log_lock_shared();
            lock_guard_sender!false;
        :: true ->
            lock_exclusive();
            log_lock_exclusive();
            lock_guard_sender!true;
        :: true ->
            try_lock_shared(success);
            if
            ::  success ->
                log_lock_shared();
                lock_guard_sender!false;
            ::  else -> goto LockFail
            fi
        :: true ->
            try_lock_exclusive(success);
            if
            ::  success ->
                log_lock_exclusive();
                lock_guard_sender!true;
            ::  else -> goto LockFail
            fi
        fi

        i = i + 1

        LockFail: skip

    ::  else -> break
    od
}

active [NPROC] proctype main_unlock() {
    // Receive a lock guard and release it
    bool exclusive;
    lock_guard_sender?exclusive
    if
    ::  exclusive ->
        log_unlock_exclusive();
        unlock_exclusive();
    ::  !exclusive ->
        log_unlock_shared();
        unlock_shared();
    fi
}
