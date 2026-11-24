Feature: Policy sources and IR are ingested without losing their meaning

  Scenario: A source at the size cap is accepted unchanged
    Given a valid UTF-8 policy source of exactly 65536 bytes
    When the source is ingested from a stream
    Then ingestion succeeds
    And the ingested bytes equal the original bytes

  Scenario: A source one byte over the cap is rejected
    Given a valid UTF-8 policy source of exactly 65537 bytes
    When the source is ingested from a stream
    Then ingestion fails because the source is too large

  Scenario: An unbounded stream is rejected without reading on
    Given an unbounded stream of policy bytes
    When the source is ingested from the stream
    Then ingestion fails because the source is too large
    And no more than 65537 bytes were read from the stream

  Scenario: Malformed UTF-8 is rejected with its offset
    Given a policy source whose byte at offset 10 starts an invalid sequence
    When the source is ingested from bytes
    Then ingestion fails with invalid UTF-8 at offset 10

  Scenario: A NUL byte is rejected with its offset
    Given a policy source with a NUL byte at offset 20
    When the source is ingested from bytes
    Then ingestion fails with a NUL byte at offset 20
