# Admission fixture: the same-tenant atom alone.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
}
