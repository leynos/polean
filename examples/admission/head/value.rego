# Admission fixture: a rule with a value and no body.
package authz

import rego.v1

default allow := false

allow := true
