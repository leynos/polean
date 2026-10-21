Feature: Pinned Regorus decides the baseline source

  Background:
    Given the baseline tenant-write source loaded into Regorus with Rego v1 and strict built-in errors

  Scenario: An owner may write an unlocked resource in their tenant
    Given the owner-unlocked write request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly true

  Scenario: A cross-tenant administrator write is denied
    Given the cross-tenant administrator write request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly false

  Scenario: An administrator may write a locked resource in their tenant
    Given the same-tenant administrator locked write request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly true

  Scenario: An owner read is denied
    Given the owner read request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly false
