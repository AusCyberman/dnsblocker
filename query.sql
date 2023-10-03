select *
FROM clients
    inner join user_clients ON clients.id = user_clients.client_id
    inner join users ON users.id = user_clients.user_id
    inner join domain_group_user on domain_group_user.user_id = users.id
    inner join domain_group on domain_group_user.domain_group_id = domain_group.id;