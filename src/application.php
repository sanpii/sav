<?php

use \PommProject\Foundation\Where;
use \Symfony\Component\HttpFoundation\Request;
use \Symfony\Component\HttpKernel\HttpKernelInterface;
use \Symfony\Component\HttpFoundation\BinaryFileResponse;

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

$app->get('/expenses/add', function (Request $request) use ($app) {
    return $app->handle(
        Request::create('/expenses/-1/edit', 'GET'),
        HttpKernelInterface::SUB_REQUEST
    );
});

$app->get('/expenses/{id}/edit', function (Request $request, $id) use ($app) {
    $map = $app['db']->getModel('\Model\ExpenseModel');

    if ($id > 0) {
        $expense = $map->findByPk(compact('id'));
        if (is_null($expense)) {
            $app->abort(404, "Achat #$id inconnu");
        }
    }
    else {
        $expense = $map->createEntity([
            'id' => $id,
            'created' => 'now',
            'serial' => '',
            'name' => '',
            'url' => '',
            'shop' => '',
            'warranty' => '',
            'price' => 0,
            'trashed' => false,
        ]);
    }

    return $app['twig']->render(
        'expense/edit.html.twig',
        compact('expense')
    );
});

$app->post('/expenses/add', function (Request $request) use ($app) {
    if ($request->files->get('invoice') === null) {
        $request->files->remove('invoice');
    }
    if ($request->files->get('notice') === null) {
        $request->files->remove('notice');
    }

    return $app->handle(
        Request::create('/expenses/-1/edit', 'POST', $request->request->all(), [], $request->files->all()),
        HttpKernelInterface::SUB_REQUEST
    );
});

$app->post('/expenses/{id}/edit', function (Request $request, $id) use ($app) {
    $map = $app['db']->getModel('\Model\ExpenseModel');
    $data = $request->request->get('expense');
    $data['warranty'] = \DateInterval::createFromDateString($data['warranty']);

    if ($id > 0) {
        $pk = compact('id');
        $expense = $map->findByPk($pk);
        if (is_null($expense)) {
            $app->abort(404, "Achat #$id inconnu");
        }
        $map->updateByPk($pk, $data);
    }
    else {
        $expense = $map->createAndSave($data);
    }

    foreach (['photo', 'invoice', 'notice'] as $type) {
        $file = $request->files->get($type);
        if ($file !== null) {
            $file->move(__DIR__ . '/../data/' . $expense->getId(), $type);
        }
    }

    $app['session']->getFlashBag()
        ->add('success', 'Achat sauvegardé');

    return $app->redirect('/');
});

$app->get('/expenses/{id}/delete', function (Request $request, $id) use ($app) {
    $map = $app['db']->getModel('\Model\ExpenseModel');

    $pk = compact('id');
    $expense = $map->findByPk($pk);
    if ($expense !== null) {
        $map->deleteOne($expense);

        $app['session']->getFlashBag()
            ->add('success', 'Achat supprimé');
    }
    else {
        $app->abort(404, "Achat #$id inconnu");
    }

    return $app->redirect('/');
});

$app->get('/expenses/{id}/trash', function (Request $request, $id) use ($app) {
    $map = $app['db']->getModel('\Model\ExpenseModel');

    $pk = compact('id');
    $expense = $map->findByPk($pk);
    if ($expense !== null) {
        $map->updateByPk($pk, ['trashed' => true]);

        $app['session']->getFlashBag()
            ->add('success', 'Achat supprimé');
    }
    else {
        $app->abort(404, "Achat #$id inconnu");
    }

    return $app->redirect('/');
});

$app->get('/expenses/{id}/untrash', function (Request $request, $id) use ($app) {
    $map = $app['db']->getModel('\Model\ExpenseModel');

    $pk = compact('id');
    $expense = $map->findByPk($pk);
    if ($expense !== null) {
        $map->updateByPk($pk, ['trashed' => false]);

        $app['session']->getFlashBag()
            ->add('success', 'Achat supprimé');
    }
    else {
        $app->abort(404, "Achat #$id inconnu");
    }

    return $app->redirect('/');
});

$app->get('/expenses/{id}/{type}', function (Request $request, $id, $type) use ($app) {
    $file = __DIR__ . "/../data/$id/$type";
    return new BinaryFileResponse($file, 200);
});

return $app;
