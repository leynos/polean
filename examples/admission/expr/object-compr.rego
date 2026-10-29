# Admission fixture: an object comprehension.
package authz

import rego.v1

default allow := false

allow if {
    {k: v | k := "a"; v := 1} == {}
}
