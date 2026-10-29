# Admission fixture: the legacy future.keywords import.
package authz

import future.keywords

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
