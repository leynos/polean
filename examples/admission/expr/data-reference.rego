# Admission fixture: a rule-body comparison against another document.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == data.tenants.current
}
