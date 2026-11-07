Feature: Settlement decides audit reports

  Scenario: A theorem of the expected proposition is accepted
    Given the audit fixture AcceptedGround
    When the settlement decides the report
    Then the settlement accepts

  Scenario: A theorem depending only on approved foundations is accepted
    Given the audit fixture AcceptedFoundations
    When the settlement decides the report
    Then the settlement accepts

  Scenario: A theorem depending on sorry is rejected
    Given the audit fixture Sorry
    When the settlement decides the report
    Then the settlement rejects with the finding SorryAxiom

  Scenario: A theorem depending on a native axiom is rejected
    Given the audit fixture Native
    When the settlement decides the report
    Then the settlement rejects with the finding NativeAxiom

  Scenario: A theorem depending on an undeclared axiom is rejected
    Given the audit fixture Undeclared
    When the settlement decides the report
    Then the settlement rejects with the finding UndeclaredAxiom

  Scenario: A theorem of another proposition is rejected
    Given the audit fixture WrongStatement
    When the settlement decides the report
    Then the settlement rejects with the finding StatementMismatch

  Scenario: An axiom root is rejected for its kind and its dependency
    Given the audit fixture AxiomRoot
    When the settlement decides the report
    Then the settlement rejects with the findings WrongKind and UndeclaredAxiom

  Scenario: A root the environment does not hold is rejected
    Given the audit fixture RootAbsent
    When the settlement decides the report
    Then the settlement rejects with the finding RootAbsent

  Scenario: A rejected replay is a completed audit that rejects
    Given the audit fixture ForgeInit
    When the settlement decides the report
    Then the settlement rejects with the finding KernelRejected

  Scenario: A report naming another toolchain is a tool failure
    Given the audit fixture AcceptedGround
    And the report names another toolchain
    When the settlement decides the report
    Then the settlement reports the tool failure ToolchainMismatch

  Scenario: A malformed report is refused before settlement
    Given a report with an unknown field
    When the adapter decodes the report
    Then the adapter refuses it as malformed
