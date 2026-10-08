package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.role == "admin"
}

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.id == input.resource.owner
    input.resource.locked == false
}
