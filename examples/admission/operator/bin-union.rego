# Admission fixture: a set union.
package authz

import rego.v1

default allow := false

allow if {
    {1} | {2} == set()
}
