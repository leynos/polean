Feature: Enforced Linux execution boundary
  Polean runs each external tool under an enforced boundary, or not at all.

  Scenario: A job cannot reach host listeners or inherit host descriptors
    Given the platform check reports every control available
    And the probe listens on a loopback port, an abstract socket, and a socket file
    When a hostile job tries to connect to each of them
    Then every attempt fails inside the job with the expected error
    And no listener records a connection
    And the job holds only its standard descriptors

  Scenario: A job that exceeds its memory limit is killed as a group
    Given the platform check reports every control available
    When a job allocates 256 MiB under a 64 MiB limit
    Then the outcome names the memory limit
    And the job's control group is empty

  Scenario: A job that floods its output is stopped at the cap
    Given the platform check reports every control available
    When a job writes 2 MiB to standard output under a 1 MiB cap
    Then the outcome names the output limit
    And exactly the first 1 MiB of the output is kept

  Scenario: A deadline reaches a process that left the process group
    Given the platform check reports every control available
    And a job has started a grandchild in a new process group
    When the job's deadline expires
    Then the outcome names the deadline
    And the job's control group is empty

  Scenario: A job writes only to its private scratch area
    Given the platform check reports every control available
    When a job writes to its scratch area, the root, its inputs, and a host path
    Then only the scratch write succeeds
    And no file from the job remains on the host

  Scenario: Missing enforcement starts no job
    Given bubblewrap is not at its configured path
    When a job is submitted
    Then the outcome is unavailable and names bubblewrap
    And no job is started
