<?php

use \PommProject\Foundation\Where;
use \Symfony\Component\HttpFoundation\Request;

$app = require __DIR__ . '/bootstrap.php';

$app->get('/', function (Request $request) use($app) {
    $page = $request->get('page', 1);
    $trashed = $request->get('trashed', false);
    $limit = $request->get('limit', 20);

    $pager = $app['db']->getModel('\Model\ExpenseModel')
        ->paginateFindWhere(
            new Where('trashed = $*', [(int)$trashed]),
            $limit,
            $page,
            'ORDER BY created DESC'
        );

    return $app['twig']->render(
        'expense/list.html.twig',
        compact('pager')
    );
});

return $app;
