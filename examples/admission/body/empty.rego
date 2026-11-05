# Admission fixture: an empty body.
package authz

import rego.v1

default allow := false

allow if {
}
