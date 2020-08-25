minikube -p brdgme docker-env | source
skaffold config set --kube-context brdgme local-cluster true