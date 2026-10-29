# Admission fixture: a bracket reference.
package authz

import rego.v1

default allow := false

allow if {
    input["action"] == "write"
}
