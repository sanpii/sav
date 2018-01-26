<?php
declare(strict_types = 1);

namespace App\Controller;

use \PommProject\Foundation\Pomm;
use \PommProject\Foundation\Where;
use \Symfony\Component\DependencyInjection\ContainerAwareInterface;
use \Symfony\Component\HttpFoundation\Request;
use \Symfony\Component\HttpFoundation\{Response, BinaryFileResponse};
use \Symfony\Component\HttpKernel\HttpKernelInterface;
use \Symfony\Component\Templating\EngineInterface;
use \Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

class Expenses implements ContainerAwareInterface
{
    use \Symfony\Bundle\FrameworkBundle\Controller\ControllerTrait;
    use \Symfony\Component\DependencyInjection\ContainerAwareTrait;

    private $pomm;
    private $templating;
    private $data_dir;

    public function __construct(EngineInterface $templating, Pomm $pomm, string $data_dir)
    {
        $this->templating = $templating;
        $this->pomm = $pomm;
        $this->data_dir = $data_dir;
    }

    public function add(): Response
    {
        return $this->forward(
            'app.controller.expenses:edit',
            ['id' => -1]
        );
    }

    public function create(Request $request): Response
    {
        if ($request->files->get('invoice') === null) {
            $request->files->remove('invoice');
        }
        if ($request->files->get('notice') === null) {
            $request->files->remove('notice');
        }

        return $this->forward(
            'app.controller.expenses:save',
            ['id' => -1]
        );
    }

    public function edit(int $id): Response
    {
        $map = $this->pomm['db']->getModel('\App\Model\ExpenseModel');

        if ($id > 0) {
            $expense = $map->findByPk(compact('id'));
            if (is_null($expense)) {
                throw new NotFoundHttpException("Achat #$id inconnu");
            }
        }
        else {
            $expense = $map->createEntity([
                'id' => $id,
                'created_at' => 'now',
                'serial' => '',
                'name' => '',
                'url' => '',
                'shop' => '',
                'warranty' => '',
                'price' => 0,
                'trashed' => false,
            ]);
        }

        return $this->render(
            'expense/edit.html.twig',
            compact('expense')
        );
    }

    public function save(Request $request, int $id): Response
    {
        $map = $this->pomm['db']->getModel('\App\Model\ExpenseModel');
        $data = $request->request->get('expense');
        $data['warranty'] = \DateInterval::createFromDateString($data['warranty']);

        if ($id > 0) {
            $pk = compact('id');
            $expense = $map->findByPk($pk);
            if (is_null($expense)) {
                throw new NotFoundHttpException("Achat #$id inconnue");
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

        $this->addFlash('success', 'Achat sauvegardé');
        return $this->redirect('/');
    }

    public function delete(int $id): Response
    {
        $map = $this->pomm['db']->getModel('\App\Model\ExpenseModel');

        $pk = compact('id');
        $expense = $map->findByPk($pk);
        if ($expense !== null) {
            $map->deleteByPk($pk);

            $this->addFlash('success', 'Achat supprimé');
        }
        else {
            throw new NotFoundHttpException("Achat #$id inconnu");
        }

        return $this->redirect('/');
    }

    public function trash(int $id): Response
    {
        $map = $this->pomm['db']->getModel('\App\Model\ExpenseModel');

        $pk = compact('id');
        $expense = $map->findByPk($pk);
        if ($expense !== null) {
            $map->updateByPk($pk, ['trashed_at' => new \DateTime()]);

            $this->addFlash('success', 'Achat jeté');
        }
        else {
            throw new NotFoundHttpException("Achat #$id inconnu");
        }

        return $this->redirect('/');
    }

    public function untrash(int $id): Response
    {
        $map = $this->pomm['db']->getModel('\App\Model\ExpenseModel');

        $pk = compact('id');
        $expense = $map->findByPk($pk);
        if ($expense !== null) {
            $map->updateByPk($pk, ['trashed_at' => null]);

            $this->addFlash('success', 'Achat recyclé');
        }
        else {
            throw new NotFoundHttpException("Achat #$id inconnu");
        }

        return $this->redirect('/');
    }

    public function media(int $id, string $type): Response
    {
        $file = $this->data_dir . "/$id/$type";
        if (!is_file($file)) {
            throw new NotFoundHttpException("$type #$id inconnu·e");
        }

        return new BinaryFileResponse($file, 200);
    }
}
