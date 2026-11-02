"""FD-19 pins. The number grammar of this format, stated as divergences."""

from forkdrift import compare_artifacts, parse_artifact


def pair(left_value, right_value):
    left = parse_artifact(
        f"interface shard-store\n\nreplicas = {left_value}\n", "run-a"
    )
    right = parse_artifact(
        f"interface shard-store\n\nreplicas = {right_value}\n", "run-b"
    )
    return compare_artifacts(left, right)


def test_underscored_number_against_plain_number_diverges():
    found = pair("1_000", "1000")
    assert len(found) == 1
    assert found[0].kind == "term-value"
    assert found[0].name == "replicas"


def test_short_underscored_number_against_plain_number_diverges():
    found = pair("1_0", "10")
    assert len(found) == 1
    assert found[0].kind == "term-value"


def test_underscored_number_against_itself_agrees():
    assert pair("1_000", "1_000") == []


def test_exponent_form_agrees_with_plain_integer():
    assert pair("1e3", "1000") == []


def test_signed_form_agrees_with_plain_integer():
    assert pair("+3", "3") == []


def test_leading_zero_form_agrees_with_plain_integer():
    assert pair("03", "3") == []


def test_leading_fraction_agrees_with_plain_fraction():
    assert pair(".5", "0.5") == []


def test_trailing_point_agrees_with_plain_integer():
    assert pair("5.", "5") == []


def test_negative_zero_agrees_with_zero():
    assert pair("-0", "0") == []


def test_huge_exponent_against_itself_agrees():
    assert pair("1e999", "1e999") == []


def test_huge_exponent_against_infinity_spelling_diverges():
    found = pair("1e999", "inf")
    assert len(found) == 1
    assert found[0].kind == "term-value"


def test_hex_form_against_decimal_diverges():
    found = pair("0x10", "16")
    assert len(found) == 1
    assert found[0].kind == "term-value"


def test_arbitrary_length_integers_compare_exactly():
    found = pair(
        "10000000000000000000000001", "10000000000000000000000002"
    )
    assert len(found) == 1
    assert found[0].kind == "term-value"


def test_arbitrary_length_integer_agrees_across_forms():
    assert pair("10000000000000000000000001", "1.0000000000000000000000001e25") == []


def test_exact_decimal_fractions_compare_exactly():
    found = pair("0.30000000000000004", "0.3")
    assert len(found) == 1
    assert found[0].kind == "term-value"


def test_underscored_text_against_itself_agrees():
    assert pair("node_1", "node_1") == []


def test_underscored_text_against_other_text_diverges():
    found = pair("node_1", "node_2")
    assert len(found) == 1
    assert found[0].kind == "term-value"


def test_an_exponent_beyond_the_arithmetics_range_compares_by_characters():
    found = pair("1e9999999999999999999", "2")
    assert len(found) == 1
    assert found[0].kind == "term-value"


def test_an_extreme_exponent_against_its_own_text_agrees():
    assert pair("1e9999999999999999999", "1e9999999999999999999") == []


def test_a_digit_from_another_script_is_text_not_a_number():
    found = pair("٣", "3")
    assert len(found) == 1
    assert found[0].kind == "term-value"
    assert pair("٣", "٣") == []


def test_trailing_point_with_exponent_agrees_with_plain_integer():
    assert pair("5.e3", "5000") == []


def test_standing_fraction_with_exponent_agrees_with_plain_integer():
    assert pair(".5e2", "50") == []


def test_capital_exponent_marker_agrees_with_plain_integer():
    assert pair("1E3", "1000") == []


def test_exponent_with_leading_zeros_agrees_with_plain_integer():
    assert pair("1e007", "10000000") == []
