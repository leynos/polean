# Admission fixture: the write-action atom alone.
package authz

import rego.v1

default allow := false

allow if {
    input.action == "write"
}
