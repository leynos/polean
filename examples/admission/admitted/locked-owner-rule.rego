# Admission fixture: the locked atom in a full rule.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.subject.role == "admin"
    input.resource.locked == true
}
