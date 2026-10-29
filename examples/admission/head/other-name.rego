# Admission fixture: a rule for another name.
package authz

import rego.v1

default allow := false

permit if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
