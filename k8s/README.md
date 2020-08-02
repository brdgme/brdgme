# Set secrets

These should be set before applying the k8s definitions, as postgres will create the DB and user.

```
kubectl create secret generic postgres-config --from-literal=POSTGRES_DB="{{database}}" --from-literal=POSTGRES_USER="{{username}}" --from-literal=POSTGRES_PASSWORD="{{password}}"
```