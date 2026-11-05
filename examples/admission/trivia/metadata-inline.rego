# Admission fixture: an inline METADATA comment.
package authz

import rego.v1 # METADATA

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
