# Admission fixture: exactly sixteen body literals.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.role == "admin"
    input.subject.id == input.resource.owner
    input.resource.locked == true
    input.resource.locked == false
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.role == "admin"
    input.subject.id == input.resource.owner
    input.resource.locked == true
    input.resource.locked == false
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.role == "admin"
    input.subject.id == input.resource.owner
}
