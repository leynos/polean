# Admission fixture: no rego.v1 import.
package authz

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
