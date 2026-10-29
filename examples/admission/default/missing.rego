# Admission fixture: no default rule.
package authz

import rego.v1

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
