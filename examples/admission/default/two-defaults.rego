# Admission fixture: a second default rule.
package authz

import rego.v1

default allow := false
default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
