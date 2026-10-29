# Admission fixture: a set intersection.
package authz

import rego.v1

default allow := false

allow if {
    {1} & {2} == set()
}
