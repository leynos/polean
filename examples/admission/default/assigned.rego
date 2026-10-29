# Admission fixture: a default using = instead of :=.
package authz

import rego.v1

default allow = false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
