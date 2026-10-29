# Admission fixture: a function rule head.
package authz

import rego.v1

default allow := false

f(x) := x
