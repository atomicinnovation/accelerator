from tasks.shared.build_measurement.cache_states import BuildCacheState


def test_a_warm_no_op_session_primes():
    assert BuildCacheState.WARM_NO_OP.primes
