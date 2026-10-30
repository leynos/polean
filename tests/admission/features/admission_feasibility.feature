Feature: Complete-module admission for polean.tenant-write.v0

  The walk decides what a module means; the recognizer decides whether the
  source is written as that meaning. A source the walk admits can still be
  refused, and a refusal must name the stage that saw the fault.

  Scenario: The baseline is admitted with its committed IR
    Given the polean.tenant-write.v0 profile contract
    And the baseline tenant-write source
    When the admission spike classifies it
    Then it is admitted with the IR in examples/tenant-write/policy.json

  Scenario: A metadata annotation is rejected although Regorus discards it
    Given the polean.tenant-write.v0 profile contract
    And the baseline source preceded by a METADATA comment block
    When the admission spike classifies it
    Then the AST walk alone admits it
    And it is rejected by the recognizer with code trivia.metadata

  Scenario: A parenthesised comparison is caught by the bounded source check
    Given the polean.tenant-write.v0 profile contract
    And the baseline source with one comparison wrapped in parentheses
    When the admission spike classifies it
    Then the AST walk alone admits it
    And it is rejected by the recognizer with code token.mismatch