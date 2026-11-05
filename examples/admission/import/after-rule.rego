# Admission fixture: an import after the first rule.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}

import data.foo
