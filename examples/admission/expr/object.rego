# Admission fixture: an object comparison.
package authz

import rego.v1

default allow := false

allow if {
    {} == {}
}
