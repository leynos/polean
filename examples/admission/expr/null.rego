# Admission fixture: a null comparison.
package authz

import rego.v1

default allow := false

allow if {
    null == null
}
