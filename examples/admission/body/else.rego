# Admission fixture: an else body.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
} else := false
