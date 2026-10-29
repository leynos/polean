# Admission fixture: a number outside the representable range.
package authz

import rego.v1

default allow := false

allow if {
    1e999 == 1e999
}
